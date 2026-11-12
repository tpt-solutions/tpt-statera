// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Policy layer (todo 2A): rounding modes with explicit dust destinations.
//!
//! Principle 2 (spec §3): every rounding mode declares where the dust goes. There is no
//! default and no silent drop. [`RoundingPolicy`] has no `Default` and both fields are
//! mandatory.
//!
//! ## Decision (docs/ARCHITECTURE.md AD-4, followed as recommended)
//!
//! * **`Floor`** is the only mode permitted. Its dust is non-negative and is retained by
//!   the sender, so the protocol never overcharges.
//! * **`Ceiling`** and **`HalfEven`** exist in the enum and carry mandatory destinations, but
//!   [`RoundingPolicy::new`] rejects them with [`PolicyError::ModeNotPermitted`]. Both can
//!   overcharge the user by a sub-unit amount. Permitting them requires a sub-unit refund
//!   accumulator, which is not designed yet.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;

use tpt_statera_core::{
    validate_transfer, SystemState, TransferProposal, ValidationOutcome, BPS_DENOMINATOR,
};

/// Rounding mode for the fee computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum RoundingMode {
    /// Round toward zero. Dust is non-negative.
    Floor,
    /// Round toward +infinity. Overcharges the user by a sub-unit amount. Not permitted yet.
    Ceiling,
    /// Round to nearest, ties to even. Can overcharge by a sub-unit amount. Not permitted yet.
    HalfEven,
}

/// Where the sub-unit rounding remainder is routed. Mandatory for every policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DustDestination {
    /// The remainder stays with the sender (never debited).
    SenderRetains,
    /// The remainder is credited to the treasury.
    Treasury,
    /// The remainder is destroyed (accounted as burned).
    Burn,
}

/// A rounding policy: a mode and its dust destination. Both are required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundingPolicy {
    mode: RoundingMode,
    dust_destination: DustDestination,
}

/// Errors from policy construction and fee computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    /// The requested rounding mode is not permitted in this release.
    ModeNotPermitted(RoundingMode),
    /// The dust destination is not valid for the mode.
    DestinationNotValid(RoundingMode, DustDestination),
    /// The fee computation does not fit in `u64`.
    Overflow,
    /// The fee rate exceeds 100% (10 000 bps).
    FeeRateOutOfRange,
    /// A tier schedule has no tiers.
    NoTiers,
    /// The first tier must start at amount 0, so that every amount has a tier.
    FirstTierNotZero,
    /// Tier thresholds must be strictly increasing.
    TiersNotIncreasing,
    /// A non-treasury share must be positive.
    ZeroShare,
    /// The shares do not sum to 10 000 bps. Carries the sum that was given.
    SharesNotExact(u128),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyError::ModeNotPermitted(m) => {
                write!(f, "rounding mode {m:?} is not permitted in this release")
            }
            PolicyError::DestinationNotValid(m, d) => {
                write!(f, "dust destination {d:?} is not valid for {m:?}")
            }
            PolicyError::Overflow => f.write_str("fee computation does not fit in u64"),
            PolicyError::FeeRateOutOfRange => f.write_str("fee rate exceeds 10000 bps"),
            PolicyError::NoTiers => f.write_str("tier schedule has no tiers"),
            PolicyError::FirstTierNotZero => f.write_str("the first tier must start at amount 0"),
            PolicyError::TiersNotIncreasing => {
                f.write_str("tier thresholds must be strictly increasing")
            }
            PolicyError::ZeroShare => f.write_str("a non-treasury share must be positive"),
            PolicyError::SharesNotExact(sum) => {
                write!(f, "shares must sum to 10000 bps, got {sum}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

pub mod split;
pub mod tiers;

pub use split::{Division, FeeShares, Share};
pub use tiers::{FeeTiers, Tier};

/// The result of [`validate_with_policy`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    /// The tier rate that was applied, in basis points.
    pub fee_rate_bps: u64,
    /// The core validator's outcome at that rate.
    pub outcome: ValidationOutcome,
    /// On `Allow`: how the rounded fee is divided among recipients. `None` on `Deny`.
    ///
    /// The core validator credits the whole fee to the treasury. The caller must move the
    /// non-treasury `parts` out of that credit, because `SystemState` has only one treasury balance.
    pub fee_split: Option<Division>,
}

/// Validate a transfer with the fee rate taken from a tier schedule, then divide the fee.
///
/// The tier is selected by `amount`. The core validator runs unchanged at that rate. On `Allow`,
/// the rounded fee is divided by `shares`. The parts always sum to the fee (`policy_split.telos`).
/// Fails with a [`PolicyError`] only if the division itself fails, which cannot happen for
/// validated `FeeShares` and a fee at most `u64::MAX`.
pub fn validate_with_policy(
    state: SystemState,
    amount: u64,
    tiers: &FeeTiers,
    shares: &FeeShares,
) -> Result<PolicyDecision, PolicyError> {
    let fee_rate_bps = tiers.rate_for(amount);
    let outcome = validate_transfer(state, TransferProposal::new(amount, fee_rate_bps));
    let fee_split = match outcome {
        ValidationOutcome::Allow { rounded_fee, .. } => Some(shares.divide(rounded_fee)?),
        ValidationOutcome::Deny => None,
    };
    Ok(PolicyDecision {
        fee_rate_bps,
        outcome,
        fee_split,
    })
}

/// The result of applying a policy to one fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeSplit {
    /// Whole-unit fee charged to the sender.
    pub fee: u64,
    /// Sub-unit remainder, in 1/10 000 of a unit. Routed to `destination`.
    pub dust_scaled: u64,
    /// Where `dust_scaled` goes.
    pub destination: DustDestination,
}

impl RoundingPolicy {
    /// Build a policy. Only `Floor` is accepted, with any of the three destinations.
    pub fn new(mode: RoundingMode, dust_destination: DustDestination) -> Result<Self, PolicyError> {
        match mode {
            RoundingMode::Floor => Ok(Self {
                mode,
                dust_destination,
            }),
            other => Err(PolicyError::ModeNotPermitted(other)),
        }
    }

    /// The rounding mode.
    pub fn mode(&self) -> RoundingMode {
        self.mode
    }

    /// The dust destination.
    pub fn dust_destination(&self) -> DustDestination {
        self.dust_destination
    }

    /// Split `amount * fee_rate_bps / 10_000` into a whole fee and a sub-unit remainder.
    ///
    /// Matches `validate_transfer` exactly for `Floor` (checked in tests).
    pub fn split_fee(&self, amount: u64, fee_rate_bps: u64) -> Result<FeeSplit, PolicyError> {
        if fee_rate_bps > 10_000 {
            return Err(PolicyError::FeeRateOutOfRange);
        }
        let numer = (amount as u128) * (fee_rate_bps as u128);
        let fee_wide = numer / BPS_DENOMINATOR;
        let dust_wide = numer % BPS_DENOMINATOR;
        let fee = u64::try_from(fee_wide).map_err(|_| PolicyError::Overflow)?;
        // dust_wide < 10_000 always, so this cast is exact.
        Ok(FeeSplit {
            fee,
            dust_scaled: dust_wide as u64,
            destination: self.dust_destination,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceiling_and_half_even_are_rejected() {
        for mode in [RoundingMode::Ceiling, RoundingMode::HalfEven] {
            for dest in [
                DustDestination::SenderRetains,
                DustDestination::Treasury,
                DustDestination::Burn,
            ] {
                assert_eq!(
                    RoundingPolicy::new(mode, dest),
                    Err(PolicyError::ModeNotPermitted(mode))
                );
            }
        }
    }

    #[test]
    fn floor_accepts_every_destination() {
        for dest in [
            DustDestination::SenderRetains,
            DustDestination::Treasury,
            DustDestination::Burn,
        ] {
            let p = RoundingPolicy::new(RoundingMode::Floor, dest).unwrap();
            assert_eq!(p.dust_destination(), dest);
        }
    }

    #[test]
    fn split_fee_rejects_over_100_percent() {
        let p = RoundingPolicy::new(RoundingMode::Floor, DustDestination::Burn).unwrap();
        assert_eq!(p.split_fee(1, 10_001), Err(PolicyError::FeeRateOutOfRange));
    }

    #[test]
    fn split_fee_floor_matches_formula() {
        let p = RoundingPolicy::new(RoundingMode::Floor, DustDestination::Treasury).unwrap();
        let s = p.split_fee(3, 1).unwrap();
        assert_eq!((s.fee, s.dust_scaled), (0, 3));
        let s = p.split_fee(10_000, 9_999).unwrap();
        assert_eq!((s.fee, s.dust_scaled), (9_999, 0));
    }
}

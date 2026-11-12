// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Deny reasons (todo 5B.01, 5B.02).
//!
//! [`validate_transfer`] is the proven function and is not changed here. It returns a bare
//! `Deny`. [`validate_transfer_explained`] wraps it and names the first check that failed, so an
//! integrator can tell a caller *why* a transfer was refused. The reason is derived by re-checking
//! the same conditions in the same order as the validator. The tests assert that the wrapper and
//! the validator never disagree.

use core::fmt;

use crate::transfer_validator::{
    validate_transfer, SystemState, TransferProposal, ValidationOutcome,
};

/// Why a transfer was denied. The first failing check, in validator order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DenyReason {
    /// `fee_rate_bps` is above 10 000 (100%). A trust-boundary violation.
    FeeRateTooHigh,
    /// The sender cannot cover `amount` on its own.
    InsufficientBalance,
    /// The sender covers `amount`, but not `amount` plus the rounded fee.
    InsufficientForFee,
    /// `recipient_balance + amount` does not fit in `u64`.
    RecipientOverflow,
    /// `treasury_balance + rounded_fee` does not fit in `u64`.
    TreasuryOverflow,
    /// The validator denied, but no check here matched. Should be unreachable. The tests assert
    /// that it never occurs. It is also the reason reported by an older server that sends a bare `D`.
    Unclassified,
}

impl DenyReason {
    /// A stable, machine-readable code. Used on the wire and in JSON. Never reused for another meaning.
    pub const fn code(self) -> &'static str {
        match self {
            DenyReason::FeeRateTooHigh => "fee_rate_too_high",
            DenyReason::InsufficientBalance => "insufficient_balance",
            DenyReason::InsufficientForFee => "insufficient_for_fee",
            DenyReason::RecipientOverflow => "recipient_overflow",
            DenyReason::TreasuryOverflow => "treasury_overflow",
            DenyReason::Unclassified => "unclassified",
        }
    }

    /// Parse a code produced by [`DenyReason::code`]. Returns `None` for an unknown code.
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "fee_rate_too_high" => DenyReason::FeeRateTooHigh,
            "insufficient_balance" => DenyReason::InsufficientBalance,
            "insufficient_for_fee" => DenyReason::InsufficientForFee,
            "recipient_overflow" => DenyReason::RecipientOverflow,
            "treasury_overflow" => DenyReason::TreasuryOverflow,
            "unclassified" => DenyReason::Unclassified,
            _ => return None,
        })
    }
}

impl fmt::Display for DenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            DenyReason::FeeRateTooHigh => "fee rate exceeds 10000 bps",
            DenyReason::InsufficientBalance => "sender balance is below the amount",
            DenyReason::InsufficientForFee => "sender balance is below the amount plus the fee",
            DenyReason::RecipientOverflow => "recipient balance would overflow u64",
            DenyReason::TreasuryOverflow => "treasury balance would overflow u64",
            DenyReason::Unclassified => "denied for an unclassified reason",
        };
        f.write_str(text)
    }
}

impl fmt::Display for ValidationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationOutcome::Allow {
                rounded_fee,
                dust_retained_by_sender,
                ..
            } => write!(
                f,
                "allow: fee {rounded_fee}, dust {dust_retained_by_sender} retained by sender"
            ),
            ValidationOutcome::Deny => f.write_str("deny"),
        }
    }
}

/// Validate a transfer and, on denial, say why.
///
/// `Ok` carries exactly the value [`validate_transfer`] returns on `Allow`. `Err` carries the
/// first failing check. `Err` is returned only when `validate_transfer` returns `Deny`.
pub fn validate_transfer_explained(
    state: SystemState,
    proposal: TransferProposal,
) -> Result<ValidationOutcome, DenyReason> {
    match validate_transfer(state, proposal) {
        allow @ ValidationOutcome::Allow { .. } => Ok(allow),
        ValidationOutcome::Deny => Err(classify(state, proposal)),
    }
}

/// Name the first failing check. Mirrors `validate_transfer` steps 0–5, in the same order.
fn classify(state: SystemState, proposal: TransferProposal) -> DenyReason {
    const U64_MAX: u128 = u64::MAX as u128;
    if proposal.fee_rate_bps > 10_000 {
        return DenyReason::FeeRateTooHigh;
    }
    if state.sender_balance < proposal.amount {
        return DenyReason::InsufficientBalance;
    }
    let fee = (proposal.amount as u128) * (proposal.fee_rate_bps as u128) / crate::BPS_DENOMINATOR;
    if (proposal.amount as u128) + fee > state.sender_balance as u128 {
        return DenyReason::InsufficientForFee;
    }
    if (state.recipient_balance as u128) + (proposal.amount as u128) > U64_MAX {
        return DenyReason::RecipientOverflow;
    }
    if (state.treasury_balance as u128) + fee > U64_MAX {
        return DenyReason::TreasuryOverflow;
    }
    DenyReason::Unclassified
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(s: u64, r: u64, t: u64) -> SystemState {
        SystemState {
            sender_balance: s,
            recipient_balance: r,
            treasury_balance: t,
        }
    }

    fn tp(amount: u64, fee_rate_bps: u64) -> TransferProposal {
        TransferProposal {
            amount,
            fee_rate_bps,
        }
    }

    #[test]
    fn each_deny_path_has_its_reason() {
        assert_eq!(
            validate_transfer_explained(st(100, 0, 0), tp(1, 10_001)),
            Err(DenyReason::FeeRateTooHigh)
        );
        assert_eq!(
            validate_transfer_explained(st(10, 0, 0), tp(11, 0)),
            Err(DenyReason::InsufficientBalance)
        );
        // 100 covers 100 but not 100 + 1 fee.
        assert_eq!(
            validate_transfer_explained(st(100, 0, 0), tp(100, 100)),
            Err(DenyReason::InsufficientForFee)
        );
        assert_eq!(
            validate_transfer_explained(st(10, u64::MAX, 0), tp(1, 0)),
            Err(DenyReason::RecipientOverflow)
        );
        assert_eq!(
            validate_transfer_explained(st(10_000, 0, u64::MAX), tp(1_000, 1_000)),
            Err(DenyReason::TreasuryOverflow)
        );
    }

    #[test]
    fn allow_matches_the_validator_exactly() {
        let s = st(1_000, 5, 7);
        let p = tp(500, 100);
        assert_eq!(
            validate_transfer_explained(s, p),
            Ok(validate_transfer(s, p))
        );
    }

    #[test]
    fn explained_agrees_with_validator_on_random_inputs() {
        // Deterministic xorshift, 50 000 cases. Covers every denial path.
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let pick = |v: u64| match v % 4 {
            0 => v % 1_000,
            1 => u64::MAX - (v % 1_000),
            _ => v,
        };
        let mut denies = [0u32; 5];
        for _ in 0..50_000 {
            let s = st(pick(next()), pick(next()), pick(next()));
            let p = tp(pick(next()), next() % 10_100);
            match validate_transfer(s, p) {
                ValidationOutcome::Allow { .. } => {
                    assert_eq!(
                        validate_transfer_explained(s, p),
                        Ok(validate_transfer(s, p))
                    );
                }
                ValidationOutcome::Deny => {
                    let r = validate_transfer_explained(s, p).expect_err("must deny");
                    assert_ne!(r, DenyReason::Unclassified, "{s:?} {p:?}");
                    denies[match r {
                        DenyReason::FeeRateTooHigh => 0,
                        DenyReason::InsufficientBalance => 1,
                        DenyReason::InsufficientForFee => 2,
                        DenyReason::RecipientOverflow => 3,
                        _ => 4,
                    }] += 1;
                }
            }
        }
        // Every reason is reached, so the sweep is not vacuous.
        assert!(denies.iter().all(|&n| n > 0), "{denies:?}");
    }

    #[test]
    fn codes_round_trip() {
        for r in [
            DenyReason::FeeRateTooHigh,
            DenyReason::InsufficientBalance,
            DenyReason::InsufficientForFee,
            DenyReason::RecipientOverflow,
            DenyReason::TreasuryOverflow,
            DenyReason::Unclassified,
        ] {
            assert_eq!(DenyReason::from_code(r.code()), Some(r));
        }
        assert_eq!(DenyReason::from_code("nope"), None);
    }
}

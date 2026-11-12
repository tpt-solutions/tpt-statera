// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Tiered fee rates by transfer amount (`docs/POLICY_MODEL.md` D1, todo 2A.03).
//!
//! A schedule is an ordered list of `(min_amount, fee_rate_bps)`. The rate for a transfer is the rate
//! of the last tier whose `min_amount <= amount`. The schedule is validated when it is built, so no
//! lookup can fail: there is no default and no silent fallback.

use crate::PolicyError;

/// One tier: transfers of at least `min_amount` pay `fee_rate_bps`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tier {
    /// Smallest transfer amount this tier applies to, in whole units.
    pub min_amount: u64,
    /// Fee rate for this tier, in basis points. At most 10 000.
    pub fee_rate_bps: u64,
}

/// A validated tier schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeTiers {
    tiers: Vec<Tier>,
}

impl FeeTiers {
    /// Build a schedule. Rules, all checked here (fail closed):
    ///
    /// * at least one tier;
    /// * the first tier starts at `min_amount == 0`, so every amount has a tier;
    /// * `min_amount` is strictly increasing;
    /// * every `fee_rate_bps <= 10 000`.
    pub fn new(tiers: Vec<Tier>) -> Result<Self, PolicyError> {
        let first = tiers.first().ok_or(PolicyError::NoTiers)?;
        if first.min_amount != 0 {
            return Err(PolicyError::FirstTierNotZero);
        }
        for w in tiers.windows(2) {
            if w[1].min_amount <= w[0].min_amount {
                return Err(PolicyError::TiersNotIncreasing);
            }
        }
        if tiers.iter().any(|t| t.fee_rate_bps > 10_000) {
            return Err(PolicyError::FeeRateOutOfRange);
        }
        Ok(Self { tiers })
    }

    /// The tiers, in order.
    pub fn tiers(&self) -> &[Tier] {
        &self.tiers
    }

    /// The fee rate for `amount`. Total: every amount maps to exactly one tier, because the first
    /// tier starts at zero and the thresholds are strictly increasing.
    pub fn rate_for(&self, amount: u64) -> u64 {
        // partition_point returns the count of tiers with min_amount <= amount (always >= 1).
        let idx = self.tiers.partition_point(|t| t.min_amount <= amount);
        self.tiers[idx - 1].fee_rate_bps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule() -> FeeTiers {
        FeeTiers::new(vec![
            Tier {
                min_amount: 0,
                fee_rate_bps: 50,
            },
            Tier {
                min_amount: 1_000,
                fee_rate_bps: 25,
            },
            Tier {
                min_amount: 100_000,
                fee_rate_bps: 10,
            },
        ])
        .unwrap()
    }

    #[test]
    fn rejects_empty_schedule() {
        assert_eq!(FeeTiers::new(vec![]), Err(PolicyError::NoTiers));
    }

    #[test]
    fn rejects_schedule_that_does_not_start_at_zero() {
        let t = vec![Tier {
            min_amount: 1,
            fee_rate_bps: 5,
        }];
        assert_eq!(FeeTiers::new(t), Err(PolicyError::FirstTierNotZero));
    }

    #[test]
    fn rejects_non_increasing_thresholds() {
        let t = vec![
            Tier {
                min_amount: 0,
                fee_rate_bps: 5,
            },
            Tier {
                min_amount: 500,
                fee_rate_bps: 4,
            },
            Tier {
                min_amount: 500,
                fee_rate_bps: 3,
            },
        ];
        assert_eq!(FeeTiers::new(t), Err(PolicyError::TiersNotIncreasing));
    }

    #[test]
    fn rejects_rate_over_100_percent() {
        let t = vec![Tier {
            min_amount: 0,
            fee_rate_bps: 10_001,
        }];
        assert_eq!(FeeTiers::new(t), Err(PolicyError::FeeRateOutOfRange));
    }

    #[test]
    fn boundaries_select_the_tier_that_starts_there() {
        let s = schedule();
        assert_eq!(s.rate_for(0), 50);
        assert_eq!(s.rate_for(999), 50);
        assert_eq!(s.rate_for(1_000), 25);
        assert_eq!(s.rate_for(99_999), 25);
        assert_eq!(s.rate_for(100_000), 10);
        assert_eq!(s.rate_for(u64::MAX), 10);
    }

    #[test]
    fn random_amounts_map_to_the_last_tier_whose_threshold_is_reached() {
        // Random amounts: the fast lookup equals the rate of the last tier whose threshold is
        // <= amount, computed the slow way.
        let s = schedule();
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        for _ in 0..50_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let amount = x;
            let slow = s
                .tiers()
                .iter()
                .rev()
                .find(|t| t.min_amount <= amount)
                .unwrap()
                .fee_rate_bps;
            assert_eq!(s.rate_for(amount), slow);
        }
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Splitting a fee among recipients (`docs/POLICY_MODEL.md` D2, todo 2A.03).
//!
//! The fee is shared by basis points. Each recipient gets `floor(fee * share_bps / 10 000)`. The
//! treasury always takes the rounding remainder, so the parts sum to the fee exactly and no unit is
//! dropped. The treasury is always one of the destinations, with its own share (possibly zero).

use crate::PolicyError;

/// The denominator of basis points.
const BPS: u128 = 10_000;

/// A non-treasury recipient and its share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Share {
    /// Recipient account id.
    pub account: u64,
    /// Share in basis points. Must be positive.
    pub bps: u64,
}

/// A validated split: a treasury share plus any number of other recipients.
/// The shares sum to exactly 10 000.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeShares {
    treasury_bps: u64,
    others: Vec<Share>,
}

/// The result of dividing one fee. `treasury` already includes the rounding remainder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Division {
    /// Amount for the treasury: its floor share plus the rounding remainder.
    pub treasury: u64,
    /// Amount for each other recipient, in the order the shares were given.
    pub parts: Vec<(u64, u64)>,
}

impl FeeShares {
    /// Build a split. Rules (fail closed):
    ///
    /// * every non-treasury share is positive;
    /// * the treasury share plus all other shares sum to exactly 10 000.
    pub fn new(treasury_bps: u64, others: Vec<Share>) -> Result<Self, PolicyError> {
        if others.iter().any(|s| s.bps == 0) {
            return Err(PolicyError::ZeroShare);
        }
        let sum: u128 = treasury_bps as u128 + others.iter().map(|s| s.bps as u128).sum::<u128>();
        if sum != BPS {
            return Err(PolicyError::SharesNotExact(sum));
        }
        Ok(Self {
            treasury_bps,
            others,
        })
    }

    /// Divide `fee`. The parts plus the treasury amount equal `fee` exactly.
    pub fn divide(&self, fee: u64) -> Result<Division, PolicyError> {
        let fee = fee as u128;
        let mut parts = Vec::with_capacity(self.others.len());
        let mut floored_total: u128 = 0;
        for s in &self.others {
            let part = fee * s.bps as u128 / BPS;
            floored_total += part;
            parts.push((
                s.account,
                u64::try_from(part).map_err(|_| PolicyError::Overflow)?,
            ));
        }
        let treasury_floor = fee * self.treasury_bps as u128 / BPS;
        floored_total += treasury_floor;
        // The rounding remainder is fee minus every floor. It is below the number of recipients.
        let remainder = fee - floored_total;
        let treasury =
            u64::try_from(treasury_floor + remainder).map_err(|_| PolicyError::Overflow)?;
        Ok(Division { treasury, parts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share(account: u64, bps: u64) -> Share {
        Share { account, bps }
    }

    #[test]
    fn rejects_shares_that_do_not_sum_to_10000() {
        assert_eq!(
            FeeShares::new(5_000, vec![share(1, 4_000)]),
            Err(PolicyError::SharesNotExact(9_000))
        );
        assert_eq!(
            FeeShares::new(6_000, vec![share(1, 5_000)]),
            Err(PolicyError::SharesNotExact(11_000))
        );
    }

    #[test]
    fn rejects_zero_non_treasury_share() {
        assert_eq!(
            FeeShares::new(10_000, vec![share(1, 0)]),
            Err(PolicyError::ZeroShare)
        );
    }

    #[test]
    fn treasury_takes_the_rounding_remainder() {
        // Fee 10 split 3333 / 3333 bps to two recipients, treasury 3334 bps: floors are 3, 3, 3,
        // so the treasury takes 3 + the remainder 1 = 4.
        let s = FeeShares::new(3_334, vec![share(1, 3_333), share(2, 3_333)]).unwrap();
        let d = s.divide(10).unwrap();
        assert_eq!(d.parts, vec![(1, 3), (2, 3)]);
        assert_eq!(d.treasury, 4);
        assert_eq!(d.treasury + d.parts.iter().map(|p| p.1).sum::<u64>(), 10);
    }

    #[test]
    fn single_recipient_taking_everything_gets_the_fee_exactly() {
        let s = FeeShares::new(0, vec![share(7, 10_000)]).unwrap();
        let d = s.divide(12_345).unwrap();
        assert_eq!(d.parts, vec![(7, 12_345)]);
        assert_eq!(d.treasury, 0);
    }

    #[test]
    fn zero_fee_divides_to_zero() {
        let s = FeeShares::new(1_000, vec![share(1, 9_000)]).unwrap();
        let d = s.divide(0).unwrap();
        assert_eq!(d.treasury, 0);
        assert_eq!(d.parts, vec![(1, 0)]);
    }

    #[test]
    fn random_fees_always_sum_exactly() {
        // The parts plus the treasury amount equal the fee for every random input (conservation of fee).
        let s = FeeShares::new(
            1_234,
            vec![share(1, 2_000), share(2, 3_766), share(3, 3_000)],
        )
        .unwrap();
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..100_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let fee = x % 1_000_000_000_000;
            let d = s.divide(fee).unwrap();
            let total: u64 = d.treasury + d.parts.iter().map(|p| p.1).sum::<u64>();
            assert_eq!(total, fee);
            // No recipient receives more than the fee.
            for (_, part) in &d.parts {
                assert!(*part <= fee);
            }
        }
    }
}

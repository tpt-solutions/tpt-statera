// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Linear vesting calculator (todo 1B.07, feeds EC4).
//!
//! Target invariant (spec §2, tokenomics row):
//! `claimable <= (elapsed / total_time) * total_allocation`, with Floor rounding.
//! Truncation dust is **never** released to the claimant; it stays in the
//! unvested pool, so `new_claimed + pool_remaining == total_allocation` holds exactly.

/// Errors returned by the vesting calculator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VestingError {
    /// `total_time` was zero, so the schedule is undefined.
    ZeroDuration,
    /// `already_claimed` exceeds `total_allocation`.
    ClaimedExceedsAllocation,
}

/// Result of a vesting computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VestingOutcome {
    /// Amount the claimant may withdraw now.
    pub claimable: u64,
    /// Cumulative amount claimed after this claim.
    pub new_claimed: u64,
    /// Amount still held by the schedule (unvested + truncation dust).
    pub pool_remaining: u64,
    /// Fraction of a unit lost to Floor truncation, in units of `1 / total_time`.
    /// Always `< total_time`. Stays in `pool_remaining`.
    pub truncation_dust: u64,
}

/// Cumulative vested amount at `elapsed` (Floor-rounded). `elapsed` is clamped to `total_time`.
pub fn vested_at(
    total_allocation: u64,
    total_time: u64,
    elapsed: u64,
) -> Result<u64, VestingError> {
    if total_time == 0 {
        return Err(VestingError::ZeroDuration);
    }
    let e = elapsed.min(total_time) as u128;
    let numerator = e * (total_allocation as u128);
    Ok((numerator / (total_time as u128)) as u64)
}

/// Compute the claimable amount and the post-claim accounting.
///
/// Fails closed: a claimant can never receive more than the Floor-rounded vested
/// amount, and `already_claimed` never decreases.
pub fn claimable(
    total_allocation: u64,
    total_time: u64,
    elapsed: u64,
    already_claimed: u64,
) -> Result<VestingOutcome, VestingError> {
    if already_claimed > total_allocation {
        return Err(VestingError::ClaimedExceedsAllocation);
    }
    let vested = vested_at(total_allocation, total_time, elapsed)?;
    // Never unclaim: if vested < already_claimed, the claimable amount is zero.
    let new_claimed = if vested > already_claimed {
        vested
    } else {
        already_claimed
    };
    let claimable = new_claimed - already_claimed;
    let truncation_dust = {
        let e = elapsed.min(total_time) as u128;
        ((e * (total_allocation as u128)) % (total_time as u128)) as u64
    };
    Ok(VestingOutcome {
        claimable,
        new_claimed,
        pool_remaining: total_allocation - new_claimed,
        truncation_dust,
    })
}

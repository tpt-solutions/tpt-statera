// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Multi-party settlement netting (todo 1B.08, feeds EC2 + EC4).
//!
//! Invariant: the sum of all party deltas in one settlement is exactly zero.
//! Settlement is **atomic**: either every balance is updated or none is.

/// Errors returned by netting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NettingError {
    /// `balances` and `deltas` have different lengths.
    LengthMismatch,
    /// The deltas do not sum to zero.
    NonZeroSum,
    /// An intermediate `i128` sum overflowed. Denied, never wrapped.
    Overflow,
    /// A party's post-settlement balance would be negative or exceed `u64::MAX`.
    OutOfRange {
        /// Index of the offending party.
        party: usize,
    },
}

/// Check that the deltas sum to exactly zero, without overflow.
pub fn check_zero_sum(deltas: &[i128]) -> Result<(), NettingError> {
    let mut acc: i128 = 0;
    for &d in deltas {
        acc = acc.checked_add(d).ok_or(NettingError::Overflow)?;
    }
    if acc == 0 {
        Ok(())
    } else {
        Err(NettingError::NonZeroSum)
    }
}

/// Atomically apply a zero-sum settlement to `balances`.
///
/// Two passes: first validate every party and the zero-sum property, then commit.
/// On error, `balances` is unchanged.
pub fn apply_settlement(balances: &mut [u64], deltas: &[i128]) -> Result<(), NettingError> {
    if balances.len() != deltas.len() {
        return Err(NettingError::LengthMismatch);
    }
    check_zero_sum(deltas)?;

    // Pass 1: validate every new balance without mutating.
    for (i, (&b, &d)) in balances.iter().zip(deltas.iter()).enumerate() {
        let new = (b as i128).checked_add(d).ok_or(NettingError::Overflow)?;
        if new < 0 || new > u64::MAX as i128 {
            return Err(NettingError::OutOfRange { party: i });
        }
    }

    // Pass 2: commit. Every value was range-checked above.
    for (b, &d) in balances.iter_mut().zip(deltas.iter()) {
        *b = ((*b as i128) + d) as u64;
    }
    Ok(())
}

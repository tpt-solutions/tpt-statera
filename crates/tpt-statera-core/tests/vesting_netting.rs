// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Tests for the vesting calculator (todo 1B.07) and netting (todo 1B.08).

use common::Rng;
use tpt_statera_core::fixtures::{TEN_PARTY_BALANCES, TEN_PARTY_DELTAS};
use tpt_statera_core::netting::{apply_settlement, check_zero_sum, NettingError};
use tpt_statera_core::vesting::{claimable, vested_at, VestingError};
use tpt_statera_testkit as common;

#[test]
fn vesting_never_exceeds_ideal_proportion() {
    let mut rng = Rng(0x7E57_0001);
    for _ in 0..200_000 {
        let alloc = rng.u64_edge();
        let total = rng.u64_edge().max(1);
        let elapsed = rng.u64_edge();
        let v = vested_at(alloc, total, elapsed).unwrap() as u128;
        let e = elapsed.min(total) as u128;
        // v <= (e/total) * alloc  <=>  v*total <= e*alloc
        assert!(v * total as u128 <= e * alloc as u128);
        // Floor: shortfall is strictly less than one unit.
        assert!((e * alloc as u128 - v * total as u128) < total as u128);
    }
}

#[test]
fn vesting_conserves_allocation_and_keeps_dust_out_of_claimant() {
    let mut rng = Rng(0x7E57_0002);
    for _ in 0..200_000 {
        let alloc = rng.u64_edge();
        let total = rng.u64_edge().max(1);
        let elapsed = rng.u64_edge();
        let already = if alloc == 0 {
            0
        } else {
            rng.next_u64() % (alloc / 2 + 1)
        };
        let o = claimable(alloc, total, elapsed, already).unwrap();
        assert_eq!(
            o.new_claimed as u128 + o.pool_remaining as u128,
            alloc as u128
        );
        assert_eq!(o.new_claimed as u128, already as u128 + o.claimable as u128);
        assert!(o.truncation_dust < total);
    }
}

#[test]
fn vesting_full_duration_releases_entire_allocation() {
    let o = claimable(1_000, 365, 365, 0).unwrap();
    assert_eq!(o.claimable, 1_000);
    assert_eq!(o.pool_remaining, 0);
}

#[test]
fn vesting_elapsed_past_end_is_clamped() {
    let o = claimable(1_000, 100, 10_000, 0).unwrap();
    assert_eq!(o.claimable, 1_000);
}

#[test]
fn vesting_partial_truncates_and_keeps_dust_in_pool() {
    // 1000 * 1 / 3 = 333.33 -> 333 claimable; the fractional unit stays in the pool.
    let o = claimable(1_000, 3, 1, 0).unwrap();
    assert_eq!(o.claimable, 333);
    assert_eq!(o.pool_remaining, 667);
}

#[test]
fn vesting_never_unclaims() {
    let o = claimable(1_000, 100, 10, 500).unwrap();
    assert_eq!(o.claimable, 0);
    assert_eq!(o.new_claimed, 500);
}

#[test]
fn vesting_rejects_zero_duration_and_overclaim() {
    assert_eq!(claimable(100, 0, 1, 0), Err(VestingError::ZeroDuration));
    assert_eq!(
        claimable(100, 10, 1, 101),
        Err(VestingError::ClaimedExceedsAllocation)
    );
}

#[test]
fn vesting_full_u64_range_does_not_overflow() {
    let o = claimable(u64::MAX, u64::MAX, u64::MAX / 2, 0).unwrap();
    assert!(o.claimable <= u64::MAX / 2 + 1);
    let o2 = claimable(u64::MAX, u64::MAX, u64::MAX, 0).unwrap();
    assert_eq!(o2.claimable, u64::MAX);
}

#[test]
fn ten_party_fixture_is_zero_sum() {
    assert_eq!(check_zero_sum(&TEN_PARTY_DELTAS), Ok(()));
}

#[test]
fn ten_party_settlement_conserves_total_balance() {
    let mut balances = TEN_PARTY_BALANCES;
    let before: u128 = balances.iter().map(|&b| b as u128).sum();
    apply_settlement(&mut balances, &TEN_PARTY_DELTAS).unwrap();
    let after: u128 = balances.iter().map(|&b| b as u128).sum();
    assert_eq!(before, after);
}

#[test]
fn netting_rejects_nonzero_sum_and_leaves_balances_untouched() {
    let mut balances = [10u64, 10, 10];
    let deltas = [1i128, 1, 0];
    assert_eq!(
        apply_settlement(&mut balances, &deltas),
        Err(NettingError::NonZeroSum)
    );
    assert_eq!(balances, [10, 10, 10]);
}

#[test]
fn netting_is_atomic_when_a_later_party_goes_negative() {
    let mut balances = [100u64, 0];
    let deltas = [5i128, -5]; // zero-sum, but party 1 would become -5
    assert_eq!(
        apply_settlement(&mut balances, &deltas),
        Err(NettingError::OutOfRange { party: 1 })
    );
    assert_eq!(balances, [100, 0], "no partial commit");
}

#[test]
fn netting_length_mismatch_is_rejected() {
    let mut balances = [1u64, 2];
    assert_eq!(
        apply_settlement(&mut balances, &[0i128]),
        Err(NettingError::LengthMismatch)
    );
}

#[test]
fn netting_i128_overflow_is_denied_not_wrapped() {
    let deltas = [i128::MAX, 1, -i128::MAX];
    assert_eq!(check_zero_sum(&deltas), Err(NettingError::Overflow));
}

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Property tests over arbitrary valid inputs (todo 1B.04).
//! Deterministic seed; set `TPT_PROP_ITERS` to run more iterations locally.

use common::{post_sum, pre_sum, Rng};
use tpt_statera_core::{
    validate_transfer, SystemState, TransferProposal, ValidationOutcome, BPS_DENOMINATOR,
};
use tpt_statera_testkit as common;

fn iters() -> u64 {
    std::env::var("TPT_PROP_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200_000)
}

fn gen_state(rng: &mut Rng) -> SystemState {
    SystemState {
        sender_balance: rng.u64_edge(),
        recipient_balance: rng.u64_edge(),
        treasury_balance: rng.u64_edge(),
    }
}

fn gen_proposal(rng: &mut Rng) -> TransferProposal {
    TransferProposal {
        amount: rng.u64_edge(),
        fee_rate_bps: rng.u64_edge(),
    }
}

#[test]
fn conservation_directionality_and_dust_hold_for_all_allows() {
    let mut rng = Rng(0x5EED_0001);
    let mut allows = 0u64;
    for _ in 0..iters() {
        let state = gen_state(&mut rng);
        let p = gen_proposal(&mut rng);
        let out = validate_transfer(state, p);
        if let ValidationOutcome::Allow {
            new_sender_balance: ns,
            new_recipient_balance: nr,
            new_treasury_balance: nt,
            rounded_fee,
            dust_retained_by_sender: dust,
        } = out
        {
            allows += 1;
            // Strict conservation: post_sum == pre_sum.
            assert_eq!(
                post_sum(out),
                Some(pre_sum(state)),
                "conservation: {state:?} {p:?}"
            );
            // Directionality.
            assert!(ns <= state.sender_balance, "sender must not increase");
            assert!(nr >= state.recipient_balance, "recipient must not decrease");
            assert!(nt >= state.treasury_balance, "treasury must not decrease");
            // Dust is the exact remainder of the fee computation.
            let numer = p.amount as u128 * p.fee_rate_bps as u128;
            assert_eq!(dust as u128, numer % BPS_DENOMINATOR);
            assert_eq!(rounded_fee as u128, numer / BPS_DENOMINATOR);
            // Fee accounting: sender paid exactly amount + fee.
            assert_eq!(state.sender_balance - ns, p.amount + rounded_fee);
        }
    }
    assert!(allows > 0, "generator produced no Allow cases");
}

#[test]
fn validate_transfer_is_pure_and_deny_paths_never_panic() {
    let mut rng = Rng(0x5EED_0002);
    let mut denies = 0u64;
    for _ in 0..iters() {
        let state = gen_state(&mut rng);
        let p = gen_proposal(&mut rng);
        let a = validate_transfer(state, p);
        let b = validate_transfer(state, p);
        assert_eq!(a, b, "validate_transfer must be a pure function");
        if a == ValidationOutcome::Deny {
            denies += 1;
        }
    }
    assert!(denies > 0);
}

#[test]
fn returned_u64_balances_match_wide_arithmetic() {
    let mut rng = Rng(0x5EED_0003);
    for _ in 0..iters() {
        let state = gen_state(&mut rng);
        let p = gen_proposal(&mut rng);
        if let ValidationOutcome::Allow {
            new_recipient_balance,
            new_treasury_balance,
            ..
        } = validate_transfer(state, p)
        {
            assert_eq!(
                state.recipient_balance as u128 + p.amount as u128,
                new_recipient_balance as u128
            );
            assert_eq!(
                state.treasury_balance as u128
                    + (p.amount as u128 * p.fee_rate_bps as u128 / BPS_DENOMINATOR),
                new_treasury_balance as u128
            );
        }
    }
}

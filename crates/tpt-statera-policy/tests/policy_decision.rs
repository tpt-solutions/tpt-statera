// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `validate_with_policy` (todo 5B.06): tier selection, the unchanged core validator, and fee division.

use tpt_statera_core::{validate_transfer, SystemState, TransferProposal, ValidationOutcome};
use tpt_statera_policy::{validate_with_policy, FeeShares, FeeTiers, Share, Tier};

fn tiers() -> FeeTiers {
    FeeTiers::new(vec![
        Tier {
            min_amount: 0,
            fee_rate_bps: 100,
        },
        Tier {
            min_amount: 1_000,
            fee_rate_bps: 50,
        },
        Tier {
            min_amount: 1_000_000,
            fee_rate_bps: 10,
        },
    ])
    .unwrap()
}

fn shares() -> FeeShares {
    // Treasury keeps 60%, account 7 gets 25%, account 8 gets 15%.
    FeeShares::new(
        6_000,
        vec![
            Share {
                account: 7,
                bps: 2_500,
            },
            Share {
                account: 8,
                bps: 1_500,
            },
        ],
    )
    .unwrap()
}

#[test]
fn the_tier_rate_is_applied_and_matches_core() {
    let state = SystemState::new(10_000_000, 0, 0);
    for amount in [0u64, 999, 1_000, 999_999, 1_000_000] {
        let d = validate_with_policy(state, amount, &tiers(), &shares()).unwrap();
        assert_eq!(d.fee_rate_bps, tiers().rate_for(amount));
        assert_eq!(
            d.outcome,
            validate_transfer(state, TransferProposal::new(amount, d.fee_rate_bps))
        );
    }
}

#[test]
fn allow_divides_the_rounded_fee_exactly() {
    let state = SystemState::new(10_000_000, 0, 0);
    let d = validate_with_policy(state, 2_000, &tiers(), &shares()).unwrap();
    let ValidationOutcome::Allow { rounded_fee, .. } = d.outcome else {
        panic!("expected Allow");
    };
    let split = d.fee_split.expect("split on Allow");
    let parts: u64 = split.treasury + split.parts.iter().map(|(_, a)| a).sum::<u64>();
    assert_eq!(parts, rounded_fee, "parts must sum to the rounded fee");
    assert_eq!(
        split.parts.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec![7, 8]
    );
}

#[test]
fn deny_has_no_split() {
    let state = SystemState::new(5, 0, 0);
    let d = validate_with_policy(state, 1_000, &tiers(), &shares()).unwrap();
    assert_eq!(d.outcome, ValidationOutcome::Deny);
    assert!(d.fee_split.is_none());
}

#[test]
fn random_sweep_conserves_fee_and_never_overdraws() {
    // Deterministic xorshift, 20 000 cases.
    let mut x: u64 = 0xD1B5_4A32_D192_ED03;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let (t, s) = (tiers(), shares());
    for _ in 0..20_000 {
        let state = SystemState::new(next() % 5_000_000, next() % 1_000, next() % 1_000);
        let amount = next() % 2_000_000;
        let d = validate_with_policy(state, amount, &t, &s).unwrap();
        if let ValidationOutcome::Allow {
            new_sender_balance,
            new_recipient_balance,
            new_treasury_balance,
            rounded_fee,
            ..
        } = d.outcome
        {
            // Core conservation still holds at the tier rate.
            let before = state.sender_balance as u128
                + state.recipient_balance as u128
                + state.treasury_balance as u128;
            let after = new_sender_balance as u128
                + new_recipient_balance as u128
                + new_treasury_balance as u128;
            assert_eq!(before, after);
            let split = d.fee_split.unwrap();
            let parts: u64 = split.treasury + split.parts.iter().map(|(_, a)| a).sum::<u64>();
            assert_eq!(parts, rounded_fee);
        } else {
            assert!(d.fee_split.is_none());
        }
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Differential oracle tests vs. an exact-rational ideal model (todo 1B.05).
//!
//! **Finding (see todo 1D.02):** the spec states `ideal Deny => realized Deny`.
//! Under Floor rounding this does NOT hold: the realized fee is floor(ideal), so a
//! state can be ideally insolvent by a fraction of a unit yet realized-solvent.
//! These tests pin down the exact relationship instead of asserting the clause.

mod common;
use common::{ideal_allows, post_sum, pre_sum, Rng};
use tpt_statera_core::{validate_transfer, SystemState, TransferProposal, ValidationOutcome};

/// Exact integer model with the **floor** fee. Independent formulation of the rules.
fn floor_model_allows(state: SystemState, p: TransferProposal) -> bool {
    if p.fee_rate_bps > 10_000 {
        return false;
    }
    let a = p.amount as u128;
    let f = a * p.fee_rate_bps as u128 / 10_000;
    let max = u64::MAX as u128;
    (state.sender_balance as u128) >= a + f
        && (state.recipient_balance as u128) + a <= max
        && (state.treasury_balance as u128) + f <= max
}

fn gen(rng: &mut Rng) -> (SystemState, TransferProposal) {
    (
        SystemState {
            sender_balance: rng.u64_edge(),
            recipient_balance: rng.u64_edge(),
            treasury_balance: rng.u64_edge(),
        },
        TransferProposal {
            amount: rng.u64_edge(),
            fee_rate_bps: rng.u64_edge(),
        },
    )
}

#[test]
fn realized_matches_floor_model_exactly() {
    let mut rng = Rng(0xD1FF_0001);
    for _ in 0..200_000 {
        let (state, p) = gen(&mut rng);
        let realized_allows =
            matches!(validate_transfer(state, p), ValidationOutcome::Allow { .. });
        assert_eq!(
            realized_allows,
            floor_model_allows(state, p),
            "{state:?} {p:?}"
        );
    }
}

#[test]
fn ideal_allow_implies_realized_allow() {
    // Floor never charges more than the ideal fee, so ideal solvency implies realized solvency.
    let mut rng = Rng(0xD1FF_0002);
    for _ in 0..200_000 {
        let (state, p) = gen(&mut rng);
        if ideal_allows(state, p) {
            assert!(
                matches!(validate_transfer(state, p), ValidationOutcome::Allow { .. }),
                "ideal Allow but realized Deny: {state:?} {p:?}"
            );
        }
    }
}

#[test]
fn ideal_deny_to_realized_allow_is_only_the_rounding_gap() {
    // Every disagreement (ideal Deny, realized Allow) must be explained by the sub-unit
    // fee remainder: the realized Allow is inside the floor model, and no overdraft occurs.
    let mut rng = Rng(0xD1FF_0003);
    let mut gap_cases = 0u64;
    for _ in 0..200_000 {
        let (state, p) = gen(&mut rng);
        let realized = validate_transfer(state, p);
        if !ideal_allows(state, p) && matches!(realized, ValidationOutcome::Allow { .. }) {
            gap_cases += 1;
            assert!(floor_model_allows(state, p));
            if let ValidationOutcome::Allow {
                new_sender_balance, ..
            } = realized
            {
                assert!(new_sender_balance as u128 <= state.sender_balance as u128);
            }
            assert_eq!(post_sum(realized), Some(pre_sum(state)));
        }
    }
    assert!(gap_cases > 0, "generator should hit the rounding gap");
}

#[test]
fn spec_clause_counterexample_amount_1_fee_50pct_sender_1() {
    // Concrete witness that `ideal Deny => realized Deny` is false under Floor:
    // ideal total = 1 + 0.5 = 1.5 > 1 (Deny); realized fee = floor(0.5) = 0, total = 1 (Allow).
    let state = SystemState {
        sender_balance: 1,
        recipient_balance: 0,
        treasury_balance: 0,
    };
    let p = TransferProposal {
        amount: 1,
        fee_rate_bps: 5_000,
    };
    assert!(!ideal_allows(state, p));
    assert_eq!(
        validate_transfer(state, p),
        ValidationOutcome::Allow {
            new_sender_balance: 0,
            new_recipient_balance: 1,
            new_treasury_balance: 0,
            rounded_fee: 0,
            dust_retained_by_sender: 5_000,
        }
    );
}

#[test]
fn realized_fee_error_is_strictly_less_than_one_unit() {
    // Realization bound (todo 1D.01): 0 <= ideal_fee - rounded_fee < 1, scaled by 10_000.
    let mut rng = Rng(0xD1FF_0004);
    for _ in 0..200_000 {
        let state = SystemState {
            sender_balance: u64::MAX,
            recipient_balance: 0,
            treasury_balance: 0,
        };
        let p = TransferProposal {
            amount: rng.u64_edge(),
            fee_rate_bps: rng.u64_edge(),
        };
        if let ValidationOutcome::Allow {
            rounded_fee,
            dust_retained_by_sender,
            ..
        } = validate_transfer(state, p)
        {
            let scaled_ideal = p.amount as u128 * p.fee_rate_bps as u128;
            let scaled_realized = rounded_fee as u128 * 10_000;
            assert!(scaled_realized <= scaled_ideal);
            assert!(scaled_ideal - scaled_realized < 10_000);
            assert_eq!(
                scaled_ideal - scaled_realized,
                dust_retained_by_sender as u128
            );
        }
    }
}

/// Restated conservative-denial clause (decision AD-5, todo 1D.02):
///
/// `ideal Deny => realized Deny, OR the ideal shortfall is below one unit`, where the
/// ideal shortfall is `amount + numer/10000 - sender` (scaled by 10000 here).
///
/// Mirrors the linear clause in `proofs/transfer_validator.telos`, which is checked
/// by the solver only in its conjunctive part. The disjunctive form is checked here.
#[test]
fn restated_conservative_denial_clause_holds() {
    // ideal Deny has two causes: solvency (sender*10000 < amount*10000 + numer) and
    // treasury overflow (treasury*10000 + numer > MAX*10000). Each cause, when the
    // realized outcome is Allow, must be a sub-unit gap (shortfall < 10000 scaled).
    let mut rng = Rng(0xD1FF_0005);
    let mut solvency_gap = 0u64;
    let mut overflow_gap = 0u64;
    let max = u64::MAX as u128;
    for _ in 0..200_000 {
        let (state, p) = gen(&mut rng);
        if ideal_allows(state, p) {
            continue; // clause is vacuous when ideal allows
        }
        if let ValidationOutcome::Allow { .. } = validate_transfer(state, p) {
            let numer = p.amount as u128 * p.fee_rate_bps as u128;
            let a = p.amount as u128;
            let solvency_short = a * 10_000 + numer;
            let solvency_short = solvency_short as i128 - (state.sender_balance as i128) * 10_000;
            let treas_short =
                (state.treasury_balance as i128) * 10_000 + numer as i128 - (max as i128) * 10_000;
            if solvency_short > 0 {
                solvency_gap += 1;
                assert!(
                    solvency_short < 10_000,
                    "solvency shortfall >= 1 unit: {state:?} {p:?}"
                );
            } else {
                overflow_gap += 1;
                assert!(
                    treas_short > 0,
                    "ideal deny without a cause: {state:?} {p:?}"
                );
                assert!(
                    treas_short < 10_000,
                    "overflow shortfall >= 1 unit: {state:?} {p:?}"
                );
            }
        }
    }
    assert!(solvency_gap > 0, "generator should hit the solvency gap");
    assert!(overflow_gap > 0, "generator should hit the overflow gap");
}

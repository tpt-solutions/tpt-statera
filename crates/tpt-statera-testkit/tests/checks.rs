// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The testkit's own checks: the invariant checkers pass on the real validator, and they fail on wrong outcomes.

use tpt_statera_core::{validate_transfer, SystemState, TransferProposal, ValidationOutcome};
use tpt_statera_testkit::{ideal_allows, invariants, Rng};

#[test]
fn every_invariant_holds_on_random_inputs() {
    let mut rng = Rng(0x7E57_0001);
    for _ in 0..20_000 {
        let (state, proposal) = (rng.state(), rng.proposal());
        let outcome = validate_transfer(state, proposal);
        invariants::all(state, proposal, outcome)
            .unwrap_or_else(|e| panic!("{state:?} {proposal:?} -> {outcome:?}: {e}"));
    }
}

#[test]
fn ideal_allow_implies_realized_allow() {
    // Floor never charges more than the ideal fee (differential.rs). The converse is false by design:
    // under Floor the realized layer can allow where the ideal layer denies, by up to one sub-unit fee.
    let mut rng = Rng(0x7E57_0002);
    for _ in 0..20_000 {
        let (state, proposal) = (rng.state(), rng.proposal());
        if ideal_allows(state, proposal) {
            assert!(
                matches!(
                    validate_transfer(state, proposal),
                    ValidationOutcome::Allow { .. }
                ),
                "{state:?} {proposal:?}"
            );
        }
    }
}

#[test]
fn a_wrong_fee_is_caught_by_the_fee_checker() {
    let state = SystemState::new(1_000, 0, 0);
    let proposal = TransferProposal::new(500, 100); // the fee is 5, with no dust
                                                    // Internally consistent, so conservation and the exact debit hold. Only the fee is wrong.
    let wrong = ValidationOutcome::Allow {
        new_sender_balance: 494,
        new_recipient_balance: 500,
        new_treasury_balance: 6,
        rounded_fee: 6,
        dust_retained_by_sender: 0,
    };
    assert!(invariants::conserves_value(state, wrong).is_ok());
    assert!(invariants::debits_exactly(state, proposal, wrong).is_ok());
    assert!(invariants::floor_fee(proposal, wrong).is_err());
}

#[test]
fn value_creation_is_caught() {
    let state = SystemState::new(1_000, 0, 0);
    let minting = ValidationOutcome::Allow {
        new_sender_balance: 500,
        new_recipient_balance: 500,
        new_treasury_balance: 5,
        rounded_fee: 5,
        dust_retained_by_sender: 0,
    };
    assert!(invariants::conserves_value(state, minting).is_err());
}

#[test]
fn a_deny_satisfies_every_checker() {
    let state = SystemState::new(0, 0, 0);
    let proposal = TransferProposal::new(1, 1);
    assert!(invariants::all(state, proposal, ValidationOutcome::Deny).is_ok());
}

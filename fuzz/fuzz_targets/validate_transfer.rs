// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Fuzz `validate_transfer` with arbitrary bytes. Every outcome must satisfy the testkit invariants,
//! and the explained wrapper must agree with the validator.
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_statera_core::{validate_transfer, validate_transfer_explained, SystemState, TransferProposal, ValidationOutcome};
use tpt_statera_testkit::invariants;

fn u64_at(data: &[u8], i: usize) -> u64 {
    let mut b = [0u8; 8];
    for (k, slot) in b.iter_mut().enumerate() {
        *slot = *data.get(i * 8 + k).unwrap_or(&0);
    }
    u64::from_le_bytes(b)
}

fuzz_target!(|data: &[u8]| {
    let state = SystemState::new(u64_at(data, 0), u64_at(data, 1), u64_at(data, 2));
    let proposal = TransferProposal::new(u64_at(data, 3), u64_at(data, 4));

    let outcome = validate_transfer(state, proposal);
    if let Err(e) = invariants::all(state, proposal, outcome) {
        panic!("{state:?} {proposal:?} -> {outcome:?}: {e}");
    }

    // The explained wrapper must agree with the validator on every input.
    match (outcome, validate_transfer_explained(state, proposal)) {
        (ValidationOutcome::Allow { .. }, Ok(o)) => assert_eq!(o, outcome),
        (ValidationOutcome::Deny, Err(reason)) => {
            assert_ne!(reason, tpt_statera_core::DenyReason::Unclassified, "{state:?} {proposal:?}")
        }
        (o, r) => panic!("disagreement: {o:?} vs {r:?}"),
    }
});

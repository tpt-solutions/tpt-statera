// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Quick start (todo 5B.07): validate two transfers and print why one was denied.
//! Run with: `cargo run -p tpt-statera-core --example quickstart`

use tpt_statera_core::prelude::*;

fn main() {
    // Sender holds 1 000 units; recipient holds 0; treasury holds 0.
    let state = SystemState::new(1_000, 0, 0);

    // 500 units at a 1% fee (100 bps).
    let ok = TransferProposal::new(500, 100);
    match validate_transfer_explained(state, ok) {
        Ok(outcome) => println!("500 @ 1%:   {outcome}"),
        Err(reason) => println!("500 @ 1%:   denied: {reason}"),
    }

    // 1 000 units at a 1% fee: the sender cannot cover the fee on top of the amount.
    let bad = TransferProposal::new(1_000, 100);
    match validate_transfer_explained(state, bad) {
        Ok(outcome) => println!("1000 @ 1%:  {outcome}"),
        Err(reason) => println!("1000 @ 1%:  denied: {reason}"),
    }
}

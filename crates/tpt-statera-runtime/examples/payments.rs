// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Worked example: payments with a fixed fee (spec §2, Enterprise Payments).
//!
//! Run: `cargo run -p tpt-statera-runtime --example payments`

use tpt_statera_core::ValidationOutcome;
use tpt_statera_runtime::{Ledger, Submission, TREASURY};

fn main() {
    // A 15 bps (0.15%) fee, as in the spec's running example. Accounts: 1 = merchant
    // settlement, 2 = payee, 3 = payer.
    let ledger = Ledger::new(&[(1, 0), (2, 0), (3, 1_000_000)], 15);

    for (from, to, amount) in [(3, 2, 100_000), (3, 2, 250_000), (2, 1, 50_000)] {
        match ledger.submit(from, to, amount).expect("valid accounts") {
            Submission::Committed(ValidationOutcome::Allow {
                rounded_fee,
                dust_retained_by_sender,
                ..
            }) => println!(
                "{from} -> {to}: amount {amount}, fee {rounded_fee}, dust kept by sender {dust_retained_by_sender}/10000"
            ),
            other => println!("{from} -> {to}: amount {amount}: {other:?}"),
        }
    }

    println!(
        "balances: payer={} payee={} merchant={} treasury={}",
        ledger.balance(3).unwrap(),
        ledger.balance(2).unwrap(),
        ledger.balance(1).unwrap(),
        ledger.balance(TREASURY).unwrap()
    );
    // Conservation: the total never changes.
    assert_eq!(ledger.total().unwrap(), 1_000_000);
    println!("total conserved: {}", ledger.total().unwrap());
}

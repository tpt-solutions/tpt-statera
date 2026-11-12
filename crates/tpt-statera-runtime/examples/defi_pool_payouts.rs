// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Worked example: fee-bearing payouts from a liquidity pool (spec §2, DeFi & Smart Contracts).
//!
//! This is NOT an AMM. It shows the part tpt-statera covers: each payout out of a pool is
//! validated, fee-charged with Floor rounding, and conserved in total. Pricing, slippage and
//! oracle inputs are outside this crate and are assumed to have produced the payout amounts.
//!
//! Run: `cargo run -p tpt-statera-runtime --example defi_pool_payouts`

use tpt_statera_core::ValidationOutcome;
use tpt_statera_runtime::{Ledger, Submission, TREASURY};

const POOL: u64 = 1;
const LP_A: u64 = 2;
const LP_B: u64 = 3;

fn main() {
    // A 30 bps (0.30%) protocol fee on payouts. The pool starts with 10 000 000 units.
    let ledger = Ledger::new(&[(POOL, 10_000_000), (LP_A, 0), (LP_B, 0)], 30);
    let total_before = ledger.total().expect("no poisoned locks");

    // Payout amounts come from the (out-of-scope) pricing layer.
    let payouts = [(LP_A, 1_234_567u64), (LP_B, 2_000_001), (LP_A, 333)];
    for (to, amount) in payouts {
        match ledger.submit(POOL, to, amount).expect("accounts exist") {
            Submission::Committed(ValidationOutcome::Allow {
                rounded_fee,
                dust_retained_by_sender,
                ..
            }) => println!(
                "payout to {to}: amount {amount}, protocol fee {rounded_fee}, sub-unit dust {dust_retained_by_sender}/10000 kept by pool"
            ),
            other => println!("payout to {to}: amount {amount}: {other:?}"),
        }
    }

    let total_after = ledger.total().expect("no poisoned locks");
    assert_eq!(
        total_before, total_after,
        "pool payouts must conserve value"
    );
    println!(
        "pool={} lp_a={} lp_b={} treasury={} total conserved={}",
        ledger.balance(POOL).unwrap(),
        ledger.balance(LP_A).unwrap(),
        ledger.balance(LP_B).unwrap(),
        ledger.balance(TREASURY).unwrap(),
        total_after == total_before
    );
    println!("note: payouts above the pool's remaining balance are denied, not partially paid.");
}

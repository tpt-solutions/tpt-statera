// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Worked example: multi-party settlement netting (spec §2, Supply Chain Finance).
//!
//! Run: `cargo run -p tpt-statera-runtime --example supply_chain_netting`

use tpt_statera_core::fixtures::{TEN_PARTY_BALANCES, TEN_PARTY_DELTAS};
use tpt_statera_core::netting::{apply_settlement, NettingError};

fn main() {
    let mut balances = TEN_PARTY_BALANCES;
    let before: u128 = balances.iter().map(|&b| b as u128).sum();

    // A rejected settlement leaves every balance unchanged (atomic).
    let mut bad = TEN_PARTY_DELTAS;
    bad[0] += 1;
    assert_eq!(
        apply_settlement(&mut balances, &bad),
        Err(NettingError::NonZeroSum)
    );
    assert_eq!(balances, TEN_PARTY_BALANCES);
    println!("non-zero-sum settlement rejected; balances unchanged");

    apply_settlement(&mut balances, &TEN_PARTY_DELTAS).expect("zero-sum settlement");
    let after: u128 = balances.iter().map(|&b| b as u128).sum();
    assert_eq!(before, after);
    println!("settled 10 parties; total before {before}, after {after}");
    for (i, b) in balances.iter().enumerate() {
        println!("  party {i}: {b}");
    }
}

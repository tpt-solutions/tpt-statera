// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Worked example: linear token vesting (spec §2, Tokenomics & Vesting).
//!
//! Run: `cargo run -p tpt-statera-runtime --example tokenomics_vesting`

use tpt_statera_core::vesting::claimable;

fn main() {
    // 1,000,000 tokens vest linearly over 365 days. Claims are made at days 30, 180, 365.
    let allocation = 1_000_000u64;
    let total_days = 365u64;
    let mut claimed = 0u64;
    let mut pool_dust_ever = 0u64;

    for day in [30u64, 180, 365] {
        let o = claimable(allocation, total_days, day, claimed).expect("valid schedule");
        claimed = o.new_claimed;
        pool_dust_ever = o.truncation_dust;
        println!(
            "day {day:>3}: claimable {:>7}, cumulative claimed {:>7}, pool remaining {:>7} (truncation dust in pool: {})",
            o.claimable, o.new_claimed, o.pool_remaining, o.truncation_dust
        );
        // Conservation of the allocation at every claim.
        assert_eq!(o.new_claimed + o.pool_remaining, allocation);
    }
    println!(
        "final claimed {claimed} of {allocation}; last-claim truncation dust {pool_dust_ever}"
    );
}

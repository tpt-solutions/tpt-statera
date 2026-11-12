// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! The Floor policy must agree with `validate_transfer` on fee and dust (todo 2A.03).

use tpt_statera_core::{validate_transfer, SystemState, TransferProposal, ValidationOutcome};
use tpt_statera_policy::{DustDestination, RoundingMode, RoundingPolicy};

#[test]
fn floor_policy_matches_validate_transfer() {
    let policy = RoundingPolicy::new(RoundingMode::Floor, DustDestination::SenderRetains).unwrap();
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..100_000 {
        let amount = next() % 1_000_000_000_000;
        let bps = next() % 10_001;
        let split = policy.split_fee(amount, bps).unwrap();
        let state = SystemState {
            sender_balance: u64::MAX,
            recipient_balance: 0,
            treasury_balance: 0,
        };
        let out = validate_transfer(
            state,
            TransferProposal {
                amount,
                fee_rate_bps: bps,
            },
        );
        match out {
            ValidationOutcome::Allow {
                rounded_fee,
                dust_retained_by_sender,
                ..
            } => {
                assert_eq!(split.fee, rounded_fee);
                assert_eq!(split.dust_scaled, dust_retained_by_sender);
            }
            ValidationOutcome::Deny => panic!("sender holds u64::MAX; must allow"),
        }
    }
}

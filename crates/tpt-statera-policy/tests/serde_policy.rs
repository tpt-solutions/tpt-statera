// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Serde for the policy types (todo 5B.03). Deserialization must run the same checks as the constructors.
#![cfg(feature = "serde")]

use tpt_statera_policy::{DustDestination, FeeShares, FeeTiers, RoundingMode, RoundingPolicy};

#[test]
fn valid_tiers_and_shares_round_trip() {
    let t: FeeTiers = serde_json::from_str(
        r#"[{"min_amount":0,"fee_rate_bps":15},{"min_amount":100,"fee_rate_bps":5}]"#,
    )
    .unwrap();
    assert_eq!(t.rate_for(99), 15);
    assert_eq!(t.rate_for(100), 5);
    let json = serde_json::to_string(&t).unwrap();
    assert_eq!(serde_json::from_str::<FeeTiers>(&json).unwrap(), t);

    let s: FeeShares =
        serde_json::from_str(r#"{"treasury_bps":6000,"others":[{"account":7,"bps":4000}]}"#)
            .unwrap();
    assert_eq!(s.divide(10).unwrap().treasury, 6);
}

#[test]
fn invalid_schedules_are_rejected_at_parse_time() {
    // First tier does not start at 0.
    assert!(serde_json::from_str::<FeeTiers>(r#"[{"min_amount":5,"fee_rate_bps":1}]"#).is_err());
    // Shares do not sum to 10 000.
    assert!(serde_json::from_str::<FeeShares>(r#"{"treasury_bps":1,"others":[]}"#).is_err());
    // Unknown fields are refused.
    assert!(
        serde_json::from_str::<FeeShares>(r#"{"treasury_bps":10000,"others":[],"x":1}"#).is_err()
    );
}

#[test]
fn enums_use_snake_case() {
    assert_eq!(
        serde_json::to_string(&RoundingMode::Floor).unwrap(),
        r#""floor""#
    );
    assert_eq!(
        serde_json::to_string(&DustDestination::SenderRetains).unwrap(),
        r#""sender_retains""#
    );
    let _ = RoundingPolicy::new(RoundingMode::Floor, DustDestination::Burn).unwrap();
}

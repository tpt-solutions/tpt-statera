// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Round-trip and strictness checks for the optional `serde` feature (todo 5B.03).
#![cfg(feature = "serde")]

use tpt_statera_core::{validate_transfer, DenyReason, SystemState, TransferProposal};

#[test]
fn state_and_proposal_round_trip_through_json() {
    let s = SystemState::new(1_000, 5, 7);
    let p = TransferProposal::new(500, 100);
    let s2: SystemState = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    let p2: TransferProposal = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!((s, p), (s2, p2));
    assert!(matches!(
        validate_transfer(s2, p2),
        tpt_statera_core::ValidationOutcome::Allow { .. }
    ));
}

#[test]
fn unknown_fields_are_rejected() {
    let bad = r#"{"amount":1,"fee_rate_bps":0,"memo":"x"}"#;
    assert!(serde_json::from_str::<TransferProposal>(bad).is_err());
    let bad = r#"{"sender_balance":1,"recipient_balance":0,"treasury_balance":0,"extra":true}"#;
    assert!(serde_json::from_str::<SystemState>(bad).is_err());
}

#[test]
fn deny_reason_uses_snake_case_codes() {
    assert_eq!(
        serde_json::to_string(&DenyReason::InsufficientForFee).unwrap(),
        r#""insufficient_for_fee""#
    );
    assert_eq!(
        DenyReason::InsufficientForFee.code(),
        "insufficient_for_fee"
    );
}

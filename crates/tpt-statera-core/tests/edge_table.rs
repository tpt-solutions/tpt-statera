// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Unit-test edge table (todo 1B.03).

use tpt_statera_core::{validate_transfer, SystemState, TransferProposal, ValidationOutcome};

fn st(s: u64, r: u64, t: u64) -> SystemState {
    SystemState {
        sender_balance: s,
        recipient_balance: r,
        treasury_balance: t,
    }
}
fn tp(amount: u64, bps: u64) -> TransferProposal {
    TransferProposal {
        amount,
        fee_rate_bps: bps,
    }
}

#[test]
fn zero_amount_zero_fee_allows_no_change() {
    let out = validate_transfer(st(100, 5, 7), tp(0, 0));
    assert_eq!(
        out,
        ValidationOutcome::Allow {
            new_sender_balance: 100,
            new_recipient_balance: 5,
            new_treasury_balance: 7,
            rounded_fee: 0,
            dust_retained_by_sender: 0,
        }
    );
}

#[test]
fn zero_amount_with_fee_rate_charges_nothing() {
    match validate_transfer(st(10, 0, 0), tp(0, 10_000)) {
        ValidationOutcome::Allow {
            rounded_fee,
            new_sender_balance,
            ..
        } => {
            assert_eq!(rounded_fee, 0);
            assert_eq!(new_sender_balance, 10);
        }
        ValidationOutcome::Deny => panic!("zero-amount transfer should be allowed"),
    }
}

#[test]
fn zero_fee_debits_exactly_amount() {
    match validate_transfer(st(100, 0, 0), tp(40, 0)) {
        ValidationOutcome::Allow {
            new_sender_balance,
            new_recipient_balance,
            rounded_fee,
            ..
        } => {
            assert_eq!(new_sender_balance, 60);
            assert_eq!(new_recipient_balance, 40);
            assert_eq!(rounded_fee, 0);
        }
        ValidationOutcome::Deny => panic!("expected Allow"),
    }
}

#[test]
fn fee_rate_9999_bps_is_accepted() {
    // amount 10_000, 9999 bps -> fee 9999 exactly, no dust.
    let out = validate_transfer(st(20_000, 0, 0), tp(10_000, 9_999));
    assert!(matches!(
        out,
        ValidationOutcome::Allow {
            rounded_fee: 9_999,
            dust_retained_by_sender: 0,
            ..
        }
    ));
}

#[test]
fn fee_rate_10000_bps_is_accepted_at_boundary() {
    let out = validate_transfer(st(20_000, 0, 0), tp(10_000, 10_000));
    assert!(matches!(
        out,
        ValidationOutcome::Allow {
            rounded_fee: 10_000,
            dust_retained_by_sender: 0,
            ..
        }
    ));
}

#[test]
fn fee_rate_above_10000_bps_denies() {
    assert_eq!(
        validate_transfer(st(u64::MAX, 0, 0), tp(1, 10_001)),
        ValidationOutcome::Deny
    );
    assert_eq!(
        validate_transfer(st(u64::MAX, 0, 0), tp(0, u64::MAX)),
        ValidationOutcome::Deny
    );
}

#[test]
fn sender_exactly_solvent_allows() {
    // amount 100 + fee 10 (10%) = 110 exactly.
    let out = validate_transfer(st(110, 0, 0), tp(100, 1_000));
    assert!(matches!(
        out,
        ValidationOutcome::Allow {
            new_sender_balance: 0,
            ..
        }
    ));
}

#[test]
fn sender_short_by_one_unit_denies() {
    assert_eq!(
        validate_transfer(st(109, 0, 0), tp(100, 1_000)),
        ValidationOutcome::Deny
    );
}

#[test]
fn amount_greater_than_balance_denies() {
    assert_eq!(
        validate_transfer(st(9, 0, 0), tp(10, 0)),
        ValidationOutcome::Deny
    );
}

#[test]
fn recipient_at_u64_max_with_any_amount_denies() {
    assert_eq!(
        validate_transfer(st(10, u64::MAX, 0), tp(1, 0)),
        ValidationOutcome::Deny
    );
}

#[test]
fn recipient_at_u64_max_with_zero_amount_allows() {
    // u64::MAX + 0 fits. No overflow, no debit.
    assert!(matches!(
        validate_transfer(st(0, u64::MAX, 0), tp(0, 0)),
        ValidationOutcome::Allow {
            new_recipient_balance: u64::MAX,
            ..
        }
    ));
}

#[test]
fn treasury_at_u64_max_with_nonzero_fee_denies() {
    // fee = floor(100 * 100 / 10000) = 1; treasury MAX + 1 overflows.
    assert_eq!(
        validate_transfer(st(1_000, 0, u64::MAX), tp(100, 100)),
        ValidationOutcome::Deny
    );
}

#[test]
fn treasury_at_u64_max_with_zero_fee_allows() {
    assert!(matches!(
        validate_transfer(st(1_000, 0, u64::MAX), tp(100, 0)),
        ValidationOutcome::Allow {
            new_treasury_balance: u64::MAX,
            ..
        }
    ));
}

#[test]
fn dust_zero_when_fee_divides_evenly() {
    // 10000 * 1000 / 10000 = 1000 exactly.
    assert!(matches!(
        validate_transfer(st(100_000, 0, 0), tp(10_000, 1_000)),
        ValidationOutcome::Allow {
            dust_retained_by_sender: 0,
            ..
        }
    ));
}

#[test]
fn dust_positive_when_fee_has_remainder() {
    // 3 * 1 / 10000 -> floor 0, dust 3. Sender pays 3 units; fee 0; dust 3 stays.
    match validate_transfer(st(100, 0, 0), tp(3, 1)) {
        ValidationOutcome::Allow {
            rounded_fee,
            dust_retained_by_sender,
            new_sender_balance,
            ..
        } => {
            assert_eq!(rounded_fee, 0);
            assert_eq!(dust_retained_by_sender, 3);
            assert_eq!(new_sender_balance, 97);
        }
        ValidationOutcome::Deny => panic!("expected Allow"),
    }
}

#[test]
fn max_amount_max_fee_does_not_panic() {
    // Must deny cleanly, not wrap or panic.
    let out = validate_transfer(st(u64::MAX, 0, 0), tp(u64::MAX, 10_000));
    assert_eq!(out, ValidationOutcome::Deny);
}

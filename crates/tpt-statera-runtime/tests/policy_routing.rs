// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Fee routing in the ledger (todo 5B.08): the fee policy's tiers pick the rate, and the shares
//! are paid out of the treasury inside the same locked commit.

use std::sync::Arc;
use std::thread;

use tpt_statera_policy::{FeeShares, FeeTiers, Share, Tier};
use tpt_statera_runtime::{Ledger, LedgerError, Submission, TREASURY};

fn one_tier(bps: u64) -> FeeTiers {
    FeeTiers::new(vec![Tier {
        min_amount: 0,
        fee_rate_bps: bps,
    }])
    .unwrap()
}

fn two_tiers() -> FeeTiers {
    FeeTiers::new(vec![
        Tier {
            min_amount: 0,
            fee_rate_bps: 100,
        },
        Tier {
            min_amount: 1_000,
            fee_rate_bps: 50,
        },
    ])
    .unwrap()
}

/// Treasury keeps 60%, account 7 gets 40%.
fn shares() -> FeeShares {
    FeeShares::new(
        6_000,
        vec![Share {
            account: 7,
            bps: 4_000,
        }],
    )
    .unwrap()
}

fn total(l: &Ledger) -> u128 {
    l.total().unwrap()
}

#[test]
fn shares_are_paid_from_the_treasury_and_the_remainder_stays_there() {
    // 500 at 1%: fee 5. Account 7 takes floor(5 * 40%) = 2, so the treasury keeps 3.
    let l = Ledger::with_policy(&[(1, 1_000), (2, 0)], one_tier(100), shares()).unwrap();
    let before = total(&l);
    assert!(matches!(
        l.submit(1, 2, 500).unwrap(),
        Submission::Committed(_)
    ));
    assert_eq!(l.balance(1).unwrap(), 495);
    assert_eq!(l.balance(2).unwrap(), 500);
    assert_eq!(l.balance(7).unwrap(), 2);
    assert_eq!(l.balance(TREASURY).unwrap(), 3);
    assert_eq!(total(&l), before, "value conserved");
}

#[test]
fn the_tier_for_the_amount_sets_the_rate() {
    // 2 000 is in the second tier (50 bps): fee 10. Account 7 takes 4, the treasury 6.
    let l = Ledger::with_policy(&[(1, 10_000), (2, 0)], two_tiers(), shares()).unwrap();
    assert!(matches!(
        l.submit(1, 2, 2_000).unwrap(),
        Submission::Committed(_)
    ));
    assert_eq!(l.balance(7).unwrap(), 4);
    assert_eq!(l.balance(TREASURY).unwrap(), 6);
    assert_eq!(l.balance(1).unwrap(), 10_000 - 2_010);
}

#[test]
fn an_existing_fee_account_keeps_its_balance() {
    let l = Ledger::with_policy(&[(1, 1_000), (7, 50)], one_tier(100), shares()).unwrap();
    assert_eq!(
        l.balance(7).unwrap(),
        50,
        "opening a share account must not reset it"
    );
}

#[test]
fn a_share_cannot_be_the_treasury_or_listed_twice() {
    let treasury = FeeShares::new(
        5_000,
        vec![Share {
            account: TREASURY,
            bps: 5_000,
        }],
    )
    .unwrap();
    assert_eq!(
        Ledger::with_policy(&[(1, 1)], one_tier(0), treasury).unwrap_err(),
        LedgerError::FeeAccountIsTreasury
    );

    let twice = FeeShares::new(
        5_000,
        vec![
            Share {
                account: 7,
                bps: 2_000,
            },
            Share {
                account: 7,
                bps: 3_000,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        Ledger::with_policy(&[(1, 1)], one_tier(0), twice).unwrap_err(),
        LedgerError::DuplicateFeeAccount(7)
    );
}

#[test]
fn a_fee_account_that_would_overflow_blocks_the_whole_transfer() {
    // Account 7 has u64::MAX - 1 and is due 2 units. Nothing may change.
    let l = Ledger::with_policy(
        &[(1, 1_000), (2, 0), (7, u64::MAX - 1)],
        one_tier(100),
        shares(),
    )
    .unwrap();
    let before = total(&l);
    assert_eq!(
        l.submit(1, 2, 500).unwrap_err(),
        LedgerError::FeeAccountOverflow(7)
    );
    assert_eq!(l.balance(1).unwrap(), 1_000);
    assert_eq!(l.balance(2).unwrap(), 0);
    assert_eq!(l.balance(7).unwrap(), u64::MAX - 1);
    assert_eq!(l.balance(TREASURY).unwrap(), 0);
    assert_eq!(total(&l), before);
}

#[test]
fn many_transfers_under_policy_conserve_value() {
    let l = Ledger::with_policy(&[(1, 5_000_000), (2, 0), (3, 0)], two_tiers(), shares()).unwrap();
    let before = total(&l);
    let mut x: u64 = 0x5EED_0001;
    for _ in 0..5_000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let to = if x % 2 == 0 { 2 } else { 3 };
        let amount = x % 3_000;
        let _ = l.submit(1, to, amount);
        assert_eq!(total(&l), before);
    }
}

#[test]
fn concurrent_transfers_under_policy_conserve_value() {
    let l = Arc::new(
        Ledger::with_policy(&[(1, 10_000_000), (2, 0), (3, 0)], one_tier(50), shares()).unwrap(),
    );
    let before = total(&l);
    let handles: Vec<_> = (0..6)
        .map(|t| {
            let l = Arc::clone(&l);
            thread::spawn(move || {
                for i in 0..300u64 {
                    let to = if (t + i) % 2 == 0 { 2 } else { 3 };
                    let _ = l.submit(1, to, 1_000 + (i % 7) * 500); // fee of at least 5 at 50 bps
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(
        total(&l),
        before,
        "conservation under concurrency with fee routing"
    );
    assert!(l.balance(7).unwrap() > 0, "the fee account was paid");
}

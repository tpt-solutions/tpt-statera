// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Interception layer (todo 2B.02, 2B.03, 2B.05).
//!
//! This crate is the only code path that changes balances. [`Ledger`] keeps its balances
//! private. Every write goes through [`Ledger::submit`], which:
//!
//! 1. locks the accounts involved, in ascending id order (2B.05: serialized
//!    validation-and-commit per account, with a fixed lock order so there is no deadlock);
//! 2. reads a versioned snapshot of those accounts (2B.03);
//! 3. runs `validate_transfer` on the snapshot;
//! 4. re-checks the versions before committing (anti-TOCTOU, 2B.03). A mismatch is
//!    reported as `Stale`, not retried silently;
//! 5. commits both balances and bumps the versions.
//!
//! ## Scope (read before relying on this)
//!
//! * This is an **in-process** guarantee, enforced by Rust visibility. It is not a
//!   host-boundary guarantee. A host that holds a `Ledger` can still choose not to call
//!   `submit`, or ignore `Denied` (spec §8, Assumption 4). The host integration (CosmWasm,
//!   todo 2B.01) is not built here.
//! * Process-local. Cross-process serialization needs a database, a chain nonce, or an actor.
//! * Single-transfer atomicity only. Multi-transfer composition is out of scope (todo 2C.03).

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod service;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Mutex, MutexGuard};

use tpt_statera_core::{
    validate_transfer_explained, DenyReason, SystemState, TransferProposal, ValidationOutcome,
};
use tpt_statera_policy::{FeeShares, FeeTiers, PolicyError};

/// Account identifier.
pub type AccountId = u64;

/// Reserved account that receives fees.
pub const TREASURY: AccountId = 0;

/// Errors from ledger operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    /// The account does not exist.
    UnknownAccount(AccountId),
    /// Sender and recipient are the same account.
    SelfTransfer,
    /// A lock was poisoned by a panicking thread. Treated as fail-closed.
    Poisoned,
    /// A fee share names the treasury account. Only with a fee policy.
    FeeAccountIsTreasury,
    /// A fee share names the same account twice. Only with a fee policy.
    DuplicateFeeAccount(AccountId),
    /// A fee account cannot take its share without overflowing `u64`. The transfer is not committed.
    FeeAccountOverflow(AccountId),
    /// The fee policy could not divide the fee. Not expected for validated shares.
    Policy(PolicyError),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::UnknownAccount(id) => write!(f, "unknown account {id}"),
            LedgerError::SelfTransfer => f.write_str("sender and recipient must differ"),
            LedgerError::Poisoned => f.write_str("account lock poisoned; failing closed"),
            LedgerError::FeeAccountIsTreasury => {
                f.write_str("a fee share cannot be the treasury account")
            }
            LedgerError::DuplicateFeeAccount(id) => write!(f, "fee account {id} is listed twice"),
            LedgerError::FeeAccountOverflow(id) => {
                write!(f, "fee account {id} would overflow; transfer not committed")
            }
            LedgerError::Policy(e) => write!(f, "fee policy: {e}"),
        }
    }
}

impl std::error::Error for LedgerError {}

/// The result of a submitted transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Submission {
    /// The transfer was validated and committed.
    Committed(ValidationOutcome),
    /// The transfer was denied by validation, with the first failing check. No state changed.
    Denied(DenyReason),
    /// A snapshot version changed before commit. No state changed. The caller may resubmit.
    Stale,
}

#[derive(Debug, Clone, Copy)]
struct Account {
    balance: u64,
    version: u64,
}

/// Fee policy applied by a ledger built with [`Ledger::with_policy`] (todo 5B.08).
#[derive(Debug)]
struct Routing {
    tiers: FeeTiers,
    shares: FeeShares,
}

/// Balances behind per-account locks. Fields are private: the only write path is `submit`.
#[derive(Debug)]
pub struct Ledger {
    accounts: BTreeMap<AccountId, Mutex<Account>>,
    fee_rate_bps: u64,
    routing: Option<Routing>,
}

impl Ledger {
    /// Create a ledger with the given accounts and a fixed fee rate (trusted config, <= 10 000).
    /// The whole fee goes to the treasury.
    pub fn new(initial: &[(AccountId, u64)], fee_rate_bps: u64) -> Self {
        let mut accounts = BTreeMap::new();
        for &(id, balance) in initial {
            accounts.insert(
                id,
                Mutex::new(Account {
                    balance,
                    version: 0,
                }),
            );
        }
        accounts.entry(TREASURY).or_insert_with(|| {
            Mutex::new(Account {
                balance: 0,
                version: 0,
            })
        });
        Self {
            accounts,
            fee_rate_bps,
            routing: None,
        }
    }

    /// Create a ledger whose fees follow a policy (todo 5B.08).
    ///
    /// * The rate for each transfer is the tier rate for its amount.
    /// * The fee is validated as usual, and the treasury is first credited the whole fee.
    /// * Then each non-treasury share is paid from the treasury, inside the same locked commit.
    ///   The treasury keeps its share and the rounding remainder.
    ///
    /// Every share account is opened here with a zero balance if it is not in `initial`. A share
    /// account cannot be the treasury, and cannot appear twice.
    pub fn with_policy(
        initial: &[(AccountId, u64)],
        tiers: FeeTiers,
        shares: FeeShares,
    ) -> Result<Self, LedgerError> {
        let mut seen = BTreeSet::new();
        for share in shares.others() {
            if share.account == TREASURY {
                return Err(LedgerError::FeeAccountIsTreasury);
            }
            if !seen.insert(share.account) {
                return Err(LedgerError::DuplicateFeeAccount(share.account));
            }
        }
        let mut ledger = Ledger::new(initial, 0);
        // Open a share account only if the caller did not give it a balance.
        for share in shares.others() {
            ledger.accounts.entry(share.account).or_insert_with(|| {
                Mutex::new(Account {
                    balance: 0,
                    version: 0,
                })
            });
        }
        ledger.routing = Some(Routing { tiers, shares });
        Ok(ledger)
    }

    /// Read one balance under its account lock.
    pub fn balance(&self, id: AccountId) -> Result<u64, LedgerError> {
        let m = self
            .accounts
            .get(&id)
            .ok_or(LedgerError::UnknownAccount(id))?;
        Ok(lock(m)?.balance)
    }

    /// Sum of all balances, taken under every account lock in id order. Used for the
    /// conservation check.
    pub fn total(&self) -> Result<u128, LedgerError> {
        let guards: Vec<MutexGuard<'_, Account>> =
            self.accounts.values().map(lock).collect::<Result<_, _>>()?;
        Ok(guards.iter().map(|g| g.balance as u128).sum())
    }

    /// Validate and commit one transfer from `sender` to `recipient`. The fee goes to the
    /// treasury.
    pub fn submit(
        &self,
        sender: AccountId,
        recipient: AccountId,
        amount: u64,
    ) -> Result<Submission, LedgerError> {
        if sender == recipient {
            return Err(LedgerError::SelfTransfer);
        }
        // Every account this transfer touches: the parties, the treasury, and any fee accounts.
        let mut ids: BTreeSet<AccountId> = [sender, recipient, TREASURY].into_iter().collect();
        if let Some(routing) = &self.routing {
            ids.extend(routing.shares.others().iter().map(|s| s.account));
        }
        // Lock in ascending id order. The set dedupes when the treasury is the sender or recipient.
        let mut locks: BTreeMap<AccountId, MutexGuard<'_, Account>> = BTreeMap::new();
        for id in &ids {
            let m = self
                .accounts
                .get(id)
                .ok_or(LedgerError::UnknownAccount(*id))?;
            locks.insert(*id, lock(m)?);
        }

        // Snapshot under the locks.
        let s = *locks[&sender];
        let r = *locks[&recipient];
        let t = *locks[&TREASURY];
        let state = SystemState {
            sender_balance: s.balance,
            recipient_balance: r.balance,
            treasury_balance: t.balance,
        };
        let versions: BTreeMap<AccountId, u64> =
            locks.iter().map(|(id, g)| (*id, g.version)).collect();

        let rate = match &self.routing {
            Some(routing) => routing.tiers.rate_for(amount),
            None => self.fee_rate_bps,
        };
        let proposal = TransferProposal::new(amount, rate);
        let result = validate_transfer_explained(state, proposal);

        // Re-check versions before committing. Under the lock they cannot change, so this
        // guards against a future change that reads outside the lock (2B.03).
        let now: BTreeMap<AccountId, u64> = locks.iter().map(|(id, g)| (*id, g.version)).collect();
        if now != versions {
            return Ok(Submission::Stale);
        }

        let outcome = match result {
            Ok(outcome) => outcome,
            Err(reason) => return Ok(Submission::Denied(reason)),
        };

        if let ValidationOutcome::Allow {
            new_sender_balance,
            new_recipient_balance,
            new_treasury_balance,
            rounded_fee,
            ..
        } = outcome
        {
            // Balances after the core transfer. Fee shares are paid out of the treasury's credit.
            let mut next: BTreeMap<AccountId, u64> = BTreeMap::new();
            next.insert(sender, new_sender_balance);
            next.insert(recipient, new_recipient_balance);
            next.insert(TREASURY, new_treasury_balance);

            if let Some(routing) = &self.routing {
                let division = routing
                    .shares
                    .divide(rounded_fee)
                    .map_err(LedgerError::Policy)?;
                let mut treasury = new_treasury_balance;
                for (account, part) in &division.parts {
                    treasury = treasury
                        .checked_sub(*part)
                        .ok_or(LedgerError::Policy(PolicyError::Overflow))?;
                    let current = next.get(account).copied().unwrap_or(locks[account].balance);
                    let credited = current
                        .checked_add(*part)
                        .ok_or(LedgerError::FeeAccountOverflow(*account))?;
                    next.insert(*account, credited);
                }
                next.insert(TREASURY, treasury);
            }

            for (id, balance) in next {
                commit(&mut locks, id, balance);
            }
        }
        Ok(Submission::Committed(outcome))
    }
}

fn commit(locks: &mut BTreeMap<AccountId, MutexGuard<'_, Account>>, id: AccountId, balance: u64) {
    if let Some(g) = locks.get_mut(&id) {
        // Bump the version only when the balance changes.
        if g.balance != balance {
            g.balance = balance;
            g.version += 1;
        }
    }
}

/// Lock a mutex. A poisoned lock is reported as `Poisoned` and not recovered from.
fn lock(m: &Mutex<Account>) -> Result<MutexGuard<'_, Account>, LedgerError> {
    m.lock().map_err(|_| LedgerError::Poisoned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_transfer_moves_amount_and_fee() {
        let l = Ledger::new(&[(1, 1_000), (2, 0)], 100); // 1% fee
        match l.submit(1, 2, 500).unwrap() {
            Submission::Committed(ValidationOutcome::Allow { rounded_fee, .. }) => {
                assert_eq!(rounded_fee, 5)
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(l.balance(1).unwrap(), 495);
        assert_eq!(l.balance(2).unwrap(), 500);
        assert_eq!(l.balance(TREASURY).unwrap(), 5);
    }

    #[test]
    fn denied_transfer_changes_nothing() {
        let l = Ledger::new(&[(1, 10), (2, 0)], 0);
        assert_eq!(
            l.submit(1, 2, 11).unwrap(),
            Submission::Denied(DenyReason::InsufficientBalance)
        );
        assert_eq!(l.balance(1).unwrap(), 10);
        assert_eq!(l.balance(2).unwrap(), 0);
    }

    #[test]
    fn self_transfer_and_unknown_account_are_errors() {
        let l = Ledger::new(&[(1, 10)], 0);
        assert_eq!(l.submit(1, 1, 1), Err(LedgerError::SelfTransfer));
        assert_eq!(l.submit(1, 9, 1), Err(LedgerError::UnknownAccount(9)));
    }

    #[test]
    fn concurrent_transfers_never_overdraw_and_conserve_value() {
        // 8 threads hammer the same sender. Total value must stay constant, and the sender
        // must never go negative. Each committed amount is at most the sender's balance.
        let start = 10_000u64;
        let l = std::sync::Arc::new(Ledger::new(&[(1, start), (2, 0), (3, 0)], 50));
        let before = l.total().unwrap();
        let mut handles = Vec::new();
        for t in 0..8u64 {
            let l = std::sync::Arc::clone(&l);
            handles.push(std::thread::spawn(move || {
                let mut committed = 0u64;
                for i in 0..500u64 {
                    let to = if (t + i) % 2 == 0 { 2 } else { 3 };
                    if let Ok(Submission::Committed(_)) = l.submit(1, to, 7 + (i % 5)) {
                        committed += 1;
                    }
                }
                committed
            }));
        }
        let commits: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
        assert!(commits > 0);
        assert_eq!(l.total().unwrap(), before, "conservation under concurrency");
        assert!(l.balance(1).unwrap() <= start);
    }
}

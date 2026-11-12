// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Test helpers for code that calls `tpt-statera-core` (todo 5E.01).
//!
//! * [`Rng`]: a seeded xorshift64* generator. It has no dependencies, and its runs are reproducible.
//!   [`Rng::u64_edge`] mixes boundary values into the stream, so edge cases come up often.
//! * [`ideal_allows`]: an exact-rational oracle for the Allow/Deny decision. Differential tests compare
//!   the validator with it.
//! * [`invariants`]: checks that an `Allow` outcome conserves value, debits exactly `amount + fee`,
//!   moves balances in the right direction, and rounds the fee with Floor. Each returns `Err` with a message.
//!
//! The optional `proptest` feature adds the `strategies` module.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use tpt_statera_core::{SystemState, TransferProposal, ValidationOutcome};

/// xorshift64* — deterministic, seeded, zero dependencies.
#[derive(Debug, Clone)]
pub struct Rng(pub u64);

impl Rng {
    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value that is often a boundary value: 0, 1, 9 999, 10 000, `u64::MAX`, and so on.
    pub fn u64_edge(&mut self) -> u64 {
        const EDGES: [u64; 8] = [0, 1, 9_999, 10_000, u64::MAX - 1, u64::MAX, 1 << 63, 42];
        match self.next_u64() % 4 {
            0 => EDGES[(self.next_u64() % EDGES.len() as u64) as usize],
            1 => self.next_u64() % 1_000_000,
            _ => self.next_u64(),
        }
    }

    /// A state with edge-biased balances.
    pub fn state(&mut self) -> SystemState {
        SystemState::new(self.u64_edge(), self.u64_edge(), self.u64_edge())
    }

    /// A proposal with an edge-biased amount, and a fee rate in `0..=10_000`.
    pub fn proposal(&mut self) -> TransferProposal {
        TransferProposal::new(self.u64_edge(), self.next_u64() % 10_001)
    }
}

/// Exact ideal-layer oracle: decides Allow/Deny with the fee as an exact rational.
///
/// Scaled by 10 000 to stay in integers:
/// `sender*10000 >= amount*10000 + amount*bps` ⟺ `sender >= amount + amount*bps/10000`.
pub fn ideal_allows(state: SystemState, p: TransferProposal) -> bool {
    if p.fee_rate_bps > 10_000 {
        return false;
    }
    let a = p.amount as u128;
    let b = p.fee_rate_bps as u128;
    let scaled_need = a * 10_000 + a * b; // == (amount + ideal_fee) * 10000
    let solvent = (state.sender_balance as u128) * 10_000 >= scaled_need;
    let max = u64::MAX as u128;
    let recip_ok = (state.recipient_balance as u128) + a <= max;
    // treasury + ideal_fee <= MAX  ⟺  treasury*10000 + a*b <= MAX*10000
    let treas_ok = (state.treasury_balance as u128) * 10_000 + a * b <= max * 10_000;
    solvent && recip_ok && treas_ok
}

/// Sum of the three balances before a transfer.
pub fn pre_sum(s: SystemState) -> u128 {
    s.sender_balance as u128 + s.recipient_balance as u128 + s.treasury_balance as u128
}

/// Sum of the three balances after an `Allow`. `None` on `Deny`.
pub fn post_sum(o: ValidationOutcome) -> Option<u128> {
    match o {
        ValidationOutcome::Allow {
            new_sender_balance,
            new_recipient_balance,
            new_treasury_balance,
            ..
        } => Some(
            new_sender_balance as u128
                + new_recipient_balance as u128
                + new_treasury_balance as u128,
        ),
        ValidationOutcome::Deny => None,
    }
}

/// Invariant checkers for an outcome. Each returns `Ok(())` when the invariant holds, and `Err` with
/// the failing values otherwise. A `Deny` satisfies every checker trivially, because no state changes.
pub mod invariants {
    use super::*;

    /// On `Allow`: the sum of the balances is unchanged.
    pub fn conserves_value(state: SystemState, outcome: ValidationOutcome) -> Result<(), String> {
        match post_sum(outcome) {
            None => Ok(()),
            Some(post) if post == pre_sum(state) => Ok(()),
            Some(post) => Err(format!("sum changed: {} -> {post}", pre_sum(state))),
        }
    }

    /// On `Allow`: the sender is debited exactly `amount + rounded_fee`, the recipient is credited
    /// `amount`, and the treasury is credited `rounded_fee`.
    pub fn debits_exactly(
        state: SystemState,
        proposal: TransferProposal,
        outcome: ValidationOutcome,
    ) -> Result<(), String> {
        let ValidationOutcome::Allow {
            new_sender_balance,
            new_recipient_balance,
            new_treasury_balance,
            rounded_fee,
            ..
        } = outcome
        else {
            return Ok(());
        };
        let debit = state.sender_balance as u128 - new_sender_balance as u128;
        let expected = proposal.amount as u128 + rounded_fee as u128;
        if debit != expected {
            return Err(format!("sender debited {debit}, expected {expected}"));
        }
        let credit_r = new_recipient_balance as u128 - state.recipient_balance as u128;
        if credit_r != proposal.amount as u128 {
            return Err(format!(
                "recipient credited {credit_r}, expected {}",
                proposal.amount
            ));
        }
        let credit_t = new_treasury_balance as u128 - state.treasury_balance as u128;
        if credit_t != rounded_fee as u128 {
            return Err(format!(
                "treasury credited {credit_t}, expected {rounded_fee}"
            ));
        }
        Ok(())
    }

    /// On `Allow`: the sender only goes down, and the recipient and treasury only go up.
    pub fn directional(state: SystemState, outcome: ValidationOutcome) -> Result<(), String> {
        let ValidationOutcome::Allow {
            new_sender_balance,
            new_recipient_balance,
            new_treasury_balance,
            ..
        } = outcome
        else {
            return Ok(());
        };
        if new_sender_balance > state.sender_balance {
            return Err("sender increased".into());
        }
        if new_recipient_balance < state.recipient_balance {
            return Err("recipient decreased".into());
        }
        if new_treasury_balance < state.treasury_balance {
            return Err("treasury decreased".into());
        }
        Ok(())
    }

    /// On `Allow`: the fee is `floor(amount * fee_rate_bps / 10 000)`, and the dust is the remainder.
    pub fn floor_fee(proposal: TransferProposal, outcome: ValidationOutcome) -> Result<(), String> {
        let ValidationOutcome::Allow {
            rounded_fee,
            dust_retained_by_sender,
            ..
        } = outcome
        else {
            return Ok(());
        };
        let numer = proposal.amount as u128 * proposal.fee_rate_bps as u128;
        let want_fee = (numer / 10_000) as u64;
        let want_dust = (numer % 10_000) as u64;
        if rounded_fee != want_fee || dust_retained_by_sender != want_dust {
            return Err(format!(
                "fee {rounded_fee}/dust {dust_retained_by_sender}, expected {want_fee}/{want_dust}"
            ));
        }
        Ok(())
    }

    /// Every checker above, in one call.
    pub fn all(
        state: SystemState,
        proposal: TransferProposal,
        outcome: ValidationOutcome,
    ) -> Result<(), String> {
        conserves_value(state, outcome)?;
        debits_exactly(state, proposal, outcome)?;
        directional(state, outcome)?;
        floor_fee(proposal, outcome)
    }
}

/// Optional proptest strategies (feature `proptest`).
#[cfg(feature = "proptest")]
pub mod strategies {
    use super::*;
    use proptest::prelude::*;

    /// A balance, biased toward the boundaries of `u64`.
    pub fn balance() -> impl Strategy<Value = u64> {
        prop_oneof![
            Just(0u64),
            Just(1u64),
            Just(u64::MAX),
            Just(u64::MAX - 1),
            0u64..=1_000_000,
            any::<u64>(),
        ]
    }

    /// A state with boundary-biased balances.
    pub fn state() -> impl Strategy<Value = SystemState> {
        (balance(), balance(), balance()).prop_map(|(s, r, t)| SystemState::new(s, r, t))
    }

    /// A proposal: boundary-biased amount, fee rate in `0..=10_000`.
    pub fn proposal() -> impl Strategy<Value = TransferProposal> {
        (balance(), 0u64..=10_000).prop_map(|(a, f)| TransferProposal::new(a, f))
    }
}

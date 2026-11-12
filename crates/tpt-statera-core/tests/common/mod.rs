// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Shared helpers: a deterministic PRNG (no external deps, reproducible runs)
//! and an exact-rational ideal-layer oracle.

#![allow(dead_code)]

use tpt_statera_core::{SystemState, TransferProposal, ValidationOutcome};

/// xorshift64* — deterministic, seeded, zero dependencies.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Mix uniform values with boundary values so edge cases are hit often.
    pub fn u64_edge(&mut self) -> u64 {
        const EDGES: [u64; 8] = [0, 1, 9_999, 10_000, u64::MAX - 1, u64::MAX, 1 << 63, 42];
        match self.next_u64() % 4 {
            0 => EDGES[(self.next_u64() % EDGES.len() as u64) as usize],
            1 => self.next_u64() % 1_000_000,
            _ => self.next_u64(),
        }
    }
}

/// Exact ideal-layer oracle: decides Allow/Deny with the fee as an exact rational.
///
/// Scaled by 10_000 to stay in integers:
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

pub fn pre_sum(s: SystemState) -> u128 {
    s.sender_balance as u128 + s.recipient_balance as u128 + s.treasury_balance as u128
}

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

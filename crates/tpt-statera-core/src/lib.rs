// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! # tpt-statera-core
//!
//! Stateful transfer validation and conservation-of-value checks (Phase 1).
//!
//! ## Scope — read before relying on any guarantee
//!
//! * [`validate_transfer`] validates **one proposal against one snapshot** of
//!   [`SystemState`]. It does **not** provide atomicity. Callers must serialize
//!   validation-and-commit per account (spec §8, Assumption 2). The runtime
//!   crate that would enforce this is out of scope for Phase 1.
//! * `fee_rate_bps <= 10_000` and `sender_balance >= amount` are *trusted-input
//!   boundaries* (spec §8, Assumption 3). The function fails closed on both,
//!   but these are caller obligations, not guarantees the function discharges.
//! * The guarantees here are **target architecture** until the proofs in
//!   `tpt-statera-proof` / `tpt-telos` are discharged (see `todo.md` 1C–1D).
//!
//! ## Arithmetic policy (todo 1A.04)
//!
//! Deny-by-design, never wrap. All intermediate values are computed in `u128` or
//! `i128` and checked against the target `u64` range before narrowing. No
//! `unsafe` code. Overflow in a `u64` addition is reported as `Deny`.
//!
//! ## `no_std` posture (todo 1A.05)
//!
//! This crate is `#![no_std]` with zero dependencies and no allocation, so it can
//! be compiled for `wasm32` and CosmWasm-style hosts.

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod netting;
pub mod transfer_validator;
pub mod vesting;

pub use transfer_validator::{validate_transfer, SystemState, TransferProposal, ValidationOutcome};

/// Basis-point denominator: `10_000 bps == 100%`.
pub const BPS_DENOMINATOR: u128 = 10_000;

/// Fixtures shared between tests, property checks and the benchmark harness
/// (todo 1B.08).
pub mod fixtures {
    /// Deterministic synthetic 10-party netting scenario.
    ///
    /// Returns per-party balance deltas that sum to exactly zero. Used for the
    /// netting demo (todo 1B.08), the differential tests, and the FM benchmark
    /// (todo 1E.02).
    pub const TEN_PARTY_DELTAS: [i128; 10] = [
        1_500_000, -250_000, 0, 75_000, -1_000_000, 42, -42, 300_000, -600_000, -25_000,
    ];

    /// Pre-settlement balances. Every `balance + delta` is non-negative.
    /// Sum of [`TEN_PARTY_DELTAS`] is zero (checked in tests).
    pub const TEN_PARTY_BALANCES: [u64; 10] = [
        5_000_000, 2_000_000, 0, 10_000, 3_000_000, 1, 999, 800_000, 1_200_000, 90_000,
    ];
}

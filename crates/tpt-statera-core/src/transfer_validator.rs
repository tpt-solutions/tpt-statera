// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Stateful transfer validator (spec Appendix A, todo 1B.01–1B.02, 1B.06).

use crate::BPS_DENOMINATOR;

/// The proposed financial action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferProposal {
    /// The exact amount the recipient should receive.
    pub amount: u64,
    /// Fee in basis points (e.g. `15` = 0.15%). Trusted config, `<= 10_000`.
    pub fee_rate_bps: u64,
}

/// Snapshot of the balances of the involved parties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemState {
    /// Sender balance before the transfer.
    pub sender_balance: u64,
    /// Recipient balance before the transfer.
    pub recipient_balance: u64,
    /// Treasury balance before the transfer.
    pub treasury_balance: u64,
}

/// Result of validation, including explicit dust accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationOutcome {
    /// The transfer is allowed. All post-state balances are included.
    Allow {
        /// Sender balance after `amount + rounded_fee` is debited.
        new_sender_balance: u64,
        /// Recipient balance after `amount` is credited.
        new_recipient_balance: u64,
        /// Treasury balance after `rounded_fee` is credited.
        new_treasury_balance: u64,
        /// Floor-rounded fee charged to the sender.
        rounded_fee: u64,
        /// Fractional remainder of the fee, never debited from the sender.
        ///
        /// Renamed from `dust_refunded_to_sender` (spec §6.1): nothing is refunded —
        /// the dust is never debited in the first place.
        dust_retained_by_sender: u64,
    },
    /// The transfer is rejected. No state change occurs.
    Deny,
}

/// Validate a transfer against a single state snapshot.
///
/// # Ideal-layer contracts (QF_LRA, exact rationals)
///
/// * **Trust boundary, `requires`** — `proposal.fee_rate_bps <= 10_000`.
///   Documented caller obligation (spec §8, Assumption 3). Fails closed below.
/// * **Trust boundary, `requires`** — `state.sender_balance >= proposal.amount`.
///   Same status as above. Fails closed below.
/// * **`ensures` (no overdraft)** — on `Allow`, every post-state balance is
///   bounded below/above by its pre-state counterpart as appropriate.
/// * **`ensures` (strict conservation)** — on `Allow`,
///   `post_sum == pre_sum` where the sums are over sender, recipient and treasury.
/// * **`ensures` (overflow denial)** — if `recipient + amount > u64::MAX` or
///   `treasury + max_possible_fee > u64::MAX` (with `max_possible_fee`
///   recomputed inline from inputs), the result is `Deny`. This is a
///   postcondition, not a precondition: the function is total over it.
///
/// # Realization-layer contracts (u64 + bounded rounding)
///
/// * **`ensures`** — `|rounded_fee − ideal_fee| <= 1.0` in fee units, with
///   Floor semantics (`0 <= ideal_fee − rounded_fee < 1`). See `todo.md` 1D.01.
/// * **`ensures`** — `ideal Deny ⟹ realized Deny` is stated in the spec but is
///   **not** true under Floor rounding as written. See the crate's
///   `tests/differential.rs` and `todo.md` 1D.02 for the discrepancy and the
///   proposed restatement.
pub fn validate_transfer(state: SystemState, proposal: TransferProposal) -> ValidationOutcome {
    // 0. Trust-boundary guard (spec §8, Assumption 3). Belt-and-suspenders only.
    if proposal.fee_rate_bps > 10_000 {
        return ValidationOutcome::Deny;
    }

    // 1. Basic solvency: cannot send more than you have (trust-boundary guard).
    if state.sender_balance < proposal.amount {
        return ValidationOutcome::Deny;
    }

    // 2. Ideal fee, Floor-rounded. The remainder is `dust` (never debited).
    let numerator = (proposal.amount as u128) * (proposal.fee_rate_bps as u128);
    let rounded_fee_wide = numerator / BPS_DENOMINATOR;
    let dust_wide = numerator % BPS_DENOMINATOR;

    // 3. Total debit from sender = amount + rounded_fee (dust excluded).
    let total_deduction = (proposal.amount as u128) + rounded_fee_wide;

    // 4. Sender must cover the full debit.
    if total_deduction > (state.sender_balance as u128) {
        return ValidationOutcome::Deny;
    }

    // 5. Recipient and treasury additions must fit in u64 before any u64 addition.
    let new_recipient_wide = (state.recipient_balance as u128) + (proposal.amount as u128);
    let new_treasury_wide = (state.treasury_balance as u128) + rounded_fee_wide;
    let u64_max = u64::MAX as u128;
    if new_recipient_wide > u64_max || new_treasury_wide > u64_max {
        return ValidationOutcome::Deny;
    }

    // 6. Apply. Every narrowing cast is proven in range by steps 4–5.
    let new_sender_wide = (state.sender_balance as u128) - total_deduction;
    ValidationOutcome::Allow {
        new_sender_balance: new_sender_wide as u64,
        new_recipient_balance: new_recipient_wide as u64,
        new_treasury_balance: new_treasury_wide as u64,
        rounded_fee: rounded_fee_wide as u64,
        dust_retained_by_sender: dust_wide as u64,
    }
}

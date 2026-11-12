# Policy Model for `tpt-statera-policy` (todo 2A.01)

This is the Phase 2 policy model. Each open question has a decision, taken as the recommended option. Each
decision names the option that was not taken, so it can be reversed by a reviewer (todo 1G.03).

## Scope

The spec (§4) gives `tpt-statera-policy` three areas: fee structures, rate limits, and proportional
distribution rules. Phase 2 implements the first, in the narrow form below. The other two are deferred.

## Decisions

### D1. Fee structure: tiered flat rates by transfer amount

- A policy is an ordered list of tiers `(min_amount, fee_rate_bps)`.
- The rate for a transfer is the rate of the last tier whose `min_amount <= amount`.
- Rules, checked when the policy is built (fail closed, no default):
  - the first tier has `min_amount = 0`;
  - `min_amount` is strictly increasing;
  - every `fee_rate_bps <= 10000`;
  - at least one tier.
- Each tier's rate then goes to `validate_transfer` as an ordinary `fee_rate_bps`. The validator is unchanged.

*Not taken:* a continuous fee curve. A curve needs a rounding rule per segment, which is not designed.

### D2. Split recipients: the fee is shared by basis points, the remainder goes to the treasury

- The fee from D1 may be divided among named destinations by shares in basis points.
- Shares must sum to exactly 10000, and each share is positive.
- Each destination's part is `floor(fee × share_bps / 10000)`. The remainder of the fee, which is less than the number
  of destinations, goes to the treasury, so the parts always sum to the fee exactly.
- The treasury is always a destination, so no fee unit can be dropped.

*Not taken:* giving the remainder to the first destination. That favours one party without a stated reason.

### D3. Rate limits: deferred, not in Phase 2

- A rate limit needs time and per-account counters. The validator is a pure function of one snapshot, so it cannot
  hold them. The runtime (`tpt-statera-runtime`) owns state, so a limit belongs there.
- Deferred to the backlog (`docs/BACKLOG.md`), with the blocker named: the runtime must define a clock source
  and a per-account counter store.

*Not taken:* a rate-limit check inside `validate_transfer`. That would make the validator stateful and break the
single-snapshot proofs.

### D4. Proportional distribution: deferred, not in Phase 2

- Proportional distribution (for example, pro-rata payouts) is the netting and vesting path. Those are proven
  separately (`netting_10party.telos`, `vesting_exact.telos`). A policy rule that restates them is not needed.
- Deferred to the backlog.

*Not taken:* a generic distribution policy. It would need its own proofs before anything uses it.

### D5. Rounding: Floor only (unchanged)

- The policy crate keeps `Floor` as the only permitted rounding mode (`docs/ARCHITECTURE.md` AD-4). Its dust is
  retained by the sender, so the protocol never overcharges.

### D6. Fee routing in the ledger: named fee accounts, paid from the treasury (todo 5B.08)

- **Decision (taken on the recommendation, 2026-11-13).** A ledger built with `Ledger::with_policy` uses the
  policy's tiers for the rate and its shares for the division. The share accounts are named accounts in the
  ledger. The core validator still credits the whole fee to the treasury. The ledger then pays each share from
  the treasury, and the treasury keeps its own share and the rounding remainder.
- **Why this model.** It keeps the proven validator unchanged. Each share is an ordinary account, so the existing
  balance and conservation checks cover it. The payout is part of the same locked commit, so no other submission
  can see a half-paid fee.
- **Rules.** A share account cannot be the treasury, and cannot be listed twice. An absent share account is opened
  with zero. An existing balance is kept. If a share would overflow its account, the transfer returns
  `LedgerError::FeeAccountOverflow` and changes nothing.
- **Not taken.** Routing a fee to a share account without going through the treasury. Fee accounts that a
  transfer can name as sender or recipient in the same snapshot are allowed, because the locks cover them.
- **Not yet done.** The CosmWasm adapter does not route fee shares. The host would need its own accounts for them.

## Proof obligations for the implementation (todo 2A.03 to 2A.05)

These are what the implementation must satisfy. They are not yet checked.

1. Tier selection is total and deterministic: every amount in `u64` maps to exactly one tier.
2. Every selected rate is `<= 10000`, so the existing `fee_rate_bps` guard in `validate_transfer` is never triggered
   by a policy.
3. The split parts plus the treasury remainder equal the fee exactly (conservation of the fee).
4. Built policies cannot produce a negative balance or overflow, by the existing `validate_transfer` proofs
   (2A.04 is the telos check that the policy-side arithmetic satisfies its preconditions).
5. The FM benchmark is re-run against the policy-augmented constraint set (2A.05).

## Open after this document

- 2A.03: the implementation of D1 and D2, with property tests in the 1B.03–1B.05 style.
- 2A.04: telos proofs for obligations 1–4.
- 2A.05: re-run of the benchmark (`bench/`), with the EC2 gates.
- D3 and D4 go to the backlog as named blockers.

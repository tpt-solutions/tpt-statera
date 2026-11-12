# tpt-statera-core

Stateful transfer validation and conservation-of-value checks for `tpt-statera`.

> **Status: Phase 1, not released.** Guarantees stated here are target architecture until the
> proofs in `tpt-statera-proof` / `tpt-telos` are discharged. See the repository
> [THREAT_MODEL.md](../../THREAT_MODEL.md) before citing any claim.

## What it provides

- `validate_transfer`: validates one transfer proposal against one snapshot of `SystemState`.
  Explicit dust accounting. Deny on overflow and on insolvency.
- Linear vesting calculator.
- Atomic 10-party netting.
- Shared test fixtures.

## Properties

- `#![no_std]`, zero runtime dependencies.
- No `unsafe` code.
- Arithmetic is hand-rolled over `u128`/`i128` with explicit checks, narrowed to `u64` only after
  the range is checked. Overflow is reported as `Deny`, never wrapped.

## Scope

- `validate_transfer` does **not** provide atomicity. Callers must serialize validate-and-commit
  per account (spec §8, Assumption 2). `tpt-statera-runtime` does this in-process.
- `fee_rate_bps <= 10_000` and `sender_balance >= amount` are trusted-input boundaries
  (spec §8, Assumption 3). The function fails closed on both, but these are caller obligations.

## Testing

```sh
cargo test -p tpt-statera-core
```

Licensed under MIT OR Apache-2.0. See [CHANGELOG.md](CHANGELOG.md) for release history.

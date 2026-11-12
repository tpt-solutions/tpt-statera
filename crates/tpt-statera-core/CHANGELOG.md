# Changelog

All notable changes to `tpt-statera-core`. Claims stay inside the spec §7 caveat and
`THREAT_MODEL.md` §5. The workspace-level history is in [../../CHANGELOG.md](../../CHANGELOG.md).

## Unreleased (0.1.0 candidate)

### Added

- `validate_transfer_explained` and `DenyReason` (todo 5B.01). `validate_transfer` is unchanged.
- `prelude`, `TransferProposal::new`, `SystemState::new`, and `Display` for `DenyReason` and `ValidationOutcome`.
- Optional `serde` feature (off by default) with `deny_unknown_fields`.
- `quickstart` example (`cargo run -p tpt-statera-core --example quickstart`).
- `validate_transfer`: Floor fee, explicit dust, deny-on-overflow and deny-on-insolvency.
- Linear vesting calculator.
- Atomic 10-party netting.
- Shared test fixtures.
- `#![no_std]` with zero dependencies.

### Known limitations

- `validate_transfer` is single-snapshot. It provides no atomicity; callers must serialize
  validate-and-commit per account.
- The Fourier–Motzkin solver does not prove the disjunctive clauses (upper rounding bound, overflow denial,
  restated denial). Under `--solver z3` the full model proves them, and Rust tests cover them too.
  Vesting is proven under Z3 for one claim and two claims over time.

### Not released

- No tag has been created. Tagging needs the owner's approval.

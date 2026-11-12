# Changelog

All notable changes to `tpt-statera-policy`. The workspace-level history is in
[../../CHANGELOG.md](../../CHANGELOG.md).

## Unreleased (0.1.0 candidate)

### Added

- Rounding policy with mandatory dust destinations. `Floor` only.
- `RoundingPolicy::split_fee`, returning a `FeeSplit` (whole fee, sub-unit remainder, destination).
- Tiered fee rates: `FeeTiers` and `Tier`. The schedule is validated at construction (non-empty,
  starts at 0, strictly increasing, rates at most 10 000 bps). `rate_for` is total (todo 2A.03).
- Fee splits by basis points: `FeeShares`, `Share` and `Division`. The treasury takes the rounding
  remainder, so the parts sum to the fee exactly (todo 2A.03).
- Tests for schedule validation, tier boundaries, split validation, and random checks of the tier
  lookup and of fee conservation under splits.
- Telos proofs `policy_tiers_3.telos` (three-tier selection) and `policy_split.telos` (fee
  conservation, treasury plus two recipients). Both PASS under Z3, each with a FAIL negative control
  (todo 2A.04, partly done).

### Known limitations

- `Ceiling` and `HalfEven` are rejected by design (AD-4).
- Tiers and splits are called by the runtime `Ledger::with_policy` (todo 5B.08). They are not called by `validate_transfer` or the CosmWasm adapter.
- The telos proofs cover a three-tier schedule and a two-recipient split. The generic Rust code for
  any number of tiers or recipients is tested, not proven. Todo 2A.04 stays open for this reason.
- Rate limits and proportional distribution are not implemented (deferred in `docs/POLICY_MODEL.md`).

### Not released

- No tag has been created. Tagging needs the owner's approval.

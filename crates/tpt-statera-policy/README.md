# tpt-statera-policy

Rounding policy, tiered fee rates, and fee splits for `tpt-statera`.

> **Status: Phase 2A, not released.** Tiers and splits are implemented and tested, and the telos
> proofs cover one three-tier schedule and one two-recipient split. They are **not yet wired** into
> `validate_transfer`, the runtime, or the CosmWasm adapter. Those callers still pass a plain fee rate.

## What it provides

- `RoundingPolicy`: a rounding mode plus a mandatory dust destination. It has no `Default`, and
  both fields must be supplied. Silent dust drops are not possible.
- `RoundingPolicy::split_fee`: divides `amount * fee_rate_bps / 10 000` into a whole fee and a
  sub-unit remainder (`FeeSplit`), in 1/10 000 of a unit. The remainder is tagged with the policy's
  dust destination.
- `FeeTiers` / `Tier`: tiered flat fee rates by transfer amount. A transfer pays the rate of the last
  tier whose `min_amount` is at most the amount. The schedule is checked when it is built: it must be
  non-empty, start at 0, have strictly increasing thresholds, and every rate must be at most 10 000 bps.
  The lookup (`rate_for`) cannot fail.
- `FeeShares` / `Share` / `Division`: splits a fee among a treasury and other recipients by basis
  points. Every non-treasury share is positive, and the shares sum to exactly 10 000. Each recipient
  gets `floor(fee * bps / 10 000)`. The treasury takes its own floor plus the rounding remainder, so
  the parts always sum to the fee exactly.

## Permitted modes

- **`Floor`** is the only permitted mode. Its dust is non-negative and is retained by the sender,
  so the protocol never overcharges.
- **`Ceiling`** and **`HalfEven`** exist in the enum, but `RoundingPolicy::new` rejects them with
  `PolicyError::ModeNotPermitted`. Both can overcharge the user by a sub-unit amount. Permitting
  them needs a sub-unit refund accumulator, which is not designed yet. See `docs/ARCHITECTURE.md` AD-4.

Tiers and splits are a fee *structure*, not a rounding mode. The rounding of each fee is still
`Floor`.

## Scope

- Tiers and splits are defined in `docs/POLICY_MODEL.md` (decisions D1 and D2). Rate limits and
  proportional distribution, the other two areas in spec §4, are deferred to the backlog.
- The crate computes fees. It does not move balances. A caller has to pass the tier's rate into
  `validate_transfer`, and the split shares into its own ledger logic.
- The runtime lists this crate as a dependency, but it does not import it yet.

## Machine-checked and tested

- Telos proofs (todo 2A.04, partly done):
  - `proofs/policy_tiers_3.telos` checks the tier selection for a schedule of exactly three tiers.
  - `proofs/policy_split.telos` checks that a treasury plus two recipients receive exactly the fee.
  - Each proof has a negative control in `proofs/negative/` that must FAIL.
- Rust tests cover the generic functions, including a fixed-seed random check of the tier lookup
  against a slow reference and a random check that split parts always sum to the fee. Telos has no
  loops, so the generic `FeeTiers` and `FeeShares` code is tested, not proven.

## Testing

```sh
cargo test -p tpt-statera-policy
telos verify --solver z3 proofs/policy_tiers_3.telos
telos verify --solver z3 proofs/policy_split.telos
```

The `telos` commands need a `tpt-telos` build with `--features z3`. See [proofs/README.md](../../proofs/README.md).

Licensed under MIT OR Apache-2.0. See [CHANGELOG.md](CHANGELOG.md) for release history.

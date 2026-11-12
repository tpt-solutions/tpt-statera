# tpt-statera-policy

Rounding policy with mandatory dust destinations and fee computation for `tpt-statera`.

> **Status: Phase 2A, not released.**

## What it provides

- `RoundingPolicy`: a rounding mode plus a mandatory dust destination. It has no `Default`, and
  both fields must be supplied. Silent dust drops are not possible.
- Fee computation under the policy.

## Permitted modes

- **`Floor`** is the only permitted mode. Its dust is non-negative and is retained by the sender,
  so the protocol never overcharges.
- **`Ceiling`** and **`HalfEven`** exist in the enum, but `RoundingPolicy::new` rejects them with
  `PolicyError::ModeNotPermitted`. Both can overcharge the user by a sub-unit amount. Permitting
  them needs a sub-unit refund accumulator, which is not designed yet. See `docs/ARCHITECTURE.md` AD-4..

## Testing

```sh
cargo test -p tpt-statera-policy
```

Licensed under MIT OR Apache-2.0. See [CHANGELOG.md](CHANGELOG.md) for release history.

# tpt-statera-runtime

In-process interception layer for `tpt-statera`. It is the only code path that changes balances
in the in-process ledger.

> **Status: Phase 2B, not released.** This is not a host integration. Read the scope notes below
> before relying on it.

## What it provides

- `Ledger`: balances are private. Every write goes through `Ledger::submit`, which:
  1. locks the involved accounts in ascending id order (no deadlock);
  2. reads a versioned snapshot of those accounts;
  3. runs `validate_transfer` from `tpt-statera-core` on the snapshot;
  4. re-checks the versions before committing. A mismatch is reported as `Stale`, not retried silently;
  5. commits both balances and bumps the versions.
- `WriterService` and `serve_tcp`: a single-writer service with a line protocol for cross-process use.
- `TcpClient`: client for the service. The service requires a shared token.
- Worked examples under `examples/`: payments, tokenomics vesting, supply-chain netting, DeFi pool
  payouts, and reference settlement with an audit bundle. `ledger_server` and `ledger_client` show the
  TCP service.

## Scope

- **In-process** guarantee, enforced by Rust visibility. It is not a host-boundary guarantee. A host
  holding a `Ledger` can skip `submit` or ignore `Denied` (spec §8, Assumption 4).
- Process-local serialization. Cross-process serialization needs a database, a chain nonce, or an actor.
- Single-transfer atomicity only. Each `submit` validates one transfer. The N-step composition argument
  (todo 2C.03, in `proofs/README.md`) relies on the serialization this crate provides in-process. The
  runtime does not check its other assumptions.
- The TCP service has no TLS. Use it on loopback or a private network only.

## Running the examples

```sh
cargo run -p tpt-statera-runtime --example payments
cargo run -p tpt-statera-runtime --example reference_settlement -- out
```

The reference settlement run regenerates the proof certificate when `TPT_TELOS` points at a `telos`
binary built with `--features z3`. Without it, the run copies the committed certificate and records that
in its manifest.

## Testing

```sh
cargo test -p tpt-statera-runtime
```

Licensed under MIT OR Apache-2.0. See [CHANGELOG.md](CHANGELOG.md) for release history.

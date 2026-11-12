# tpt-statera-cosmwasm

CosmWasm adapter for `tpt-statera`. The contract's `Transfer` message is the only state-changing
path. It calls `tpt-statera-core::validate_transfer` and writes balances only on `Allow`.

> **Status: Phase 2B, not released.** Unit-tested with `cw-multi-test`. **Not yet run on a live chain.**

## What it provides

- A CosmWasm contract (`cosmwasm-std` 3.0.11). `Transfer` is the only state-changing message.
- On `Deny`, `execute` returns an error and CosmWasm reverts the whole message.
- Amounts are `Uint64` on the wire, serialized as strings, so JSON clients keep precision above 2^53.
- Balances are stored under `bal:<address>` keys. Addresses are checked with `addr_validate`.
- Builds for `wasm32` (about 371 KB).

## Trust and scope

- **Instantiate sets the fee rate and the initial balances.** This is trusted configuration. There is
  no admin message after instantiation, and funding after instantiation is not supported. This is
  deliberate, and it is not a complete token system.
- **Serialization is the chain's.** CosmWasm runs one message at a time per chain, so the in-process
  lock in `tpt-statera-runtime` is not used here. The claim depends on the chain's ordering guarantees
  (see `docs/BYPASS_ARGUMENT.md`).

## Features

- `library`: disables the exported entry points, so the crate can be used by other contracts or tests.
  Build the contract with default features.

## Building

```sh
cargo test -p tpt-statera-cosmwasm
cargo build -p tpt-statera-cosmwasm --target wasm32-unknown-unknown --release
```

Licensed under MIT OR Apache-2.0. See [CHANGELOG.md](CHANGELOG.md) for release history.

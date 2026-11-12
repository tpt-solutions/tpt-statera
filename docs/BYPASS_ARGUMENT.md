# Bypass-Resistance Argument (todo 2B.04)

**Status: in-process part satisfied; CosmWasm part partly satisfied.** This argues what the in-process runtime prevents, and what the CosmWasm adapter (`crates/tpt-statera-cosmwasm`, AD-8) inherits from the chain. It is not a formal host-level proof.

## Claim

Within the `tpt-statera-runtime` crate, no code path changes a balance except `Ledger::submit`, and
`Ledger::submit` changes a balance only after `validate_transfer` returns `Allow` on a snapshot taken
under the account locks, with the versions unchanged.

## Argument

1. **Balances are private.** `Ledger.accounts` is a private field. `Account` is a private type. No
   public method returns a mutable reference to a balance. The only public write is `submit`. The
   compiler enforces this (`#![deny(unsafe_code)]` and private visibility). There is no `unsafe` in the
   runtime that could reach the field.
2. **`submit` commits only on `Allow`.** The commit branch is reached only from
   `ValidationOutcome::Allow`. `Deny` returns `Submission::Denied` with no write. `Stale` returns with no
   write. The versions are checked before the write, under the same locks.
3. **Locks cover the whole read-validate-write.** The snapshot and the writes happen while the locks for
   the sender, recipient and treasury are held. Because the lock order is fixed, two submissions that share
   an account cannot interleave between snapshot and commit.
4. **No write outside the locks.** `commit` takes the guards it is given, and it is called only inside
   `submit`, after the snapshot.

The concurrency test (`concurrent_transfers_never_overdraw_and_conserve_value`) exercises claims 2–3 under
8 threads.

## What this does not prevent

- **Host bypass.** A host that holds a `Ledger` can choose not to call `submit`, or can ignore `Denied`.
  The Rust type system cannot force a caller. This is THREAT_MODEL A4 and T6.
- **A different balance store.** The argument covers this crate's `Ledger` only. Any other store
  (a database, chain state, another ledger instance) is outside it.
- **Two `Ledger` instances on the same balances.** Each instance has its own locks, so two instances
  can overdraw. The argument assumes one `Ledger` per balance set. This is a deployment rule, and it is
  not enforced in code.
- **Cross-process use.** Locks are process-local (THREAT_MODEL T5).
- **Panics.** A panic while a guard is held poisons the lock. `lock()` then returns `Poisoned`, and
  `submit` returns an error with no write. Any later `total()` or `balance()` also fails closed. That is
  not a bypass, but it is a liveness failure, and it is the intended behavior.

## Conditions the argument depends on

| Condition | Enforced by | If violated |
| :--- | :--- | :--- |
| One `Ledger` per balance set | deployment rule | Overdraft across instances |
| Host calls `submit` for every transfer | host integration (not built) | Balances change without validation |
| Host honors `Denied` | host integration (not built) | Denied transfer treated as success |
| Validation reads only the snapshot under the lock | code (claims 1, 3) | Covered by tests, not by a proof |

## Host-level argument for the CosmWasm adapter

The adapter's only state-changing message is `Transfer`, and its only write path is the `transfer`
function. That function writes three balances on `Allow` and returns `Err` on `Deny`. Unit tests cover
both claims: `transfer_moves_amount_and_fee_and_conserves_supply` and
`denied_transfer_errors_and_writes_nothing`.

What the chain adds, and what this argument depends on:

1. **Serial execution.** The CosmWasm docs state that the transactions in a block run in sequence, so
   two `Transfer` messages cannot interleave on the same balances. Sources:
   [CosmWasm transactions architecture](https://cosmwasm.cosmos.network/core/architecture/transactions)
   and [CosmWasm SEMANTICS.md](https://www.github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md).
2. **Atomic rollback.** An error from `execute` reverts the message, so the `Err` on `Deny` writes
   nothing. The same pages state that the error case rolls back all state changes. The simulator tests in
   `crates/tpt-statera-cosmwasm/tests/multitest.rs` check this in `cw-multi-test` 3.0.1: a denied message writes
   nothing, and a denied message in a multi-message transaction rolls back the valid message before it. This is
   a simulator check, not a chain check.
3. **No submessages.** The adapter never dispatches submessages, so the partial-revert exception for
   submessages does not apply.

### Source re-read (checked 2026-11-13)

The three claims were re-checked against the pages as they now stand, and against the pinned sources
(`cosmwasm-std` 3.0.11 and `cosmwasm-vm` 3.0.11, from the cargo registry).

- **Claim 1 (serial execution): confirmed for transactions.** The Transactions chapter
  (cosmwasm.github.io/core/architecture/transactions) says the transactions in a committed block are fed to the
  Cosmos SDK "sequentially in order", and that messages within one transaction "are executed in turn". The old
  `cosmwasm.cosmos.network` URL now redirects, so the cited link was out of date.
- **Claim 2 (error reverts): confirmed for the adapter's case.** The Semantics chapter
  (cosmwasm.github.io/core/architecture/semantics) says: "If one message fails, all later messages are skipped and
  all state changes are reverted", and that an `x/wasm` error "will revert the entire transaction". So a `Deny`
  (an `Err` from `transfer`) reverts the message and any earlier valid message in its transaction. This is a
  chain rule. The page does not spell out, for a single contract entry point, which state it reverts. `Deny`
  writes nothing before it returns `Err` in any case, so the Deny path's no-write result does not depend on the
  rollback. The simulator tests in `crates/tpt-statera-cosmwasm/tests/multitest.rs` check the rollback behaviour in
  `cw-multi-test`, not on a chain.
- **Claim 3 (no submessages): confirmed in our code.** `crates/tpt-statera-cosmwasm/src/lib.rs` builds every
  response with `Response::new()` and `add_attribute` only. There is no `SubMsg`, `add_submessage` or `reply`.
  The Reply chapter says an error does not abort the whole transaction "if and only if the `reply` is called",
  so the no-submessage property is what keeps the rollback rule above unqualified.

**Correction to the earlier text.** The IBC and cross-chain caveat was attributed to these same pages. The
Transactions and Semantics chapters do not discuss IBC. The caveat is still true for this adapter, because it
makes no cross-chain calls, but it needs its own source (the IBC documentation), which has not been read.

**The only state-changing message is `Transfer`.** `ExecuteMsg` has one variant, and `execute` routes only to
`transfer`. That is a code fact, checked by reading `lib.rs`.

What the chain does not give us, and the adapter does not supply:

- **Admin paths.** There is no admin message after instantiate. Any future admin path must be reviewed
  against this argument.
- **Instantiate trust.** The fee rate and initial balances are set by whoever instantiates the contract
  (THREAT_MODEL A3). The adapter cannot check that.
- **IBC and cross-chain calls.** Not covered by the pages above. The adapter makes no cross-chain calls, so
  this does not apply to it today.

## Snapshot integrity on CosmWasm (todo 2B.03)

The in-process runtime takes a snapshot under the account locks and re-checks the versions before it writes
(claims 1 and 3 above). On CosmWasm there is no separate snapshot to drift:

- `transfer` reads `sender`, `recipient` and `treasury` balances from `deps.storage`, calls `validate_transfer`
  on those values, and on `Allow` writes the three results back to the same `deps.storage` (see
  `crates/tpt-statera-cosmwasm/src/lib.rs`, `transfer`). The validated values and the written values come from
  one storage view inside one call.
- That view is the transaction's cache. The Transactions chapter says a transaction "only writes to a cache,
  which may be committed or rolled back on error", so no other transaction's writes appear in the middle of this
  call. The serial execution of transactions (claim 1) is what makes this hold.
- So the host-environment state that the validator saw is the state the chain commits. The check that 2B.03
  asks for is satisfied structurally, not by a runtime comparison.

This relies on the serial-execution rule (claim 1) and on the cache rule above. Both are confirmed in the chain
documentation as of 2026-11-13. It does not cover an off-chain host that holds a `Ledger`, where the version check
in `Ledger::submit` is the control.

## Remaining path

1. Re-read the cited CosmWasm pages against cosmwasm-std 3.0.11 and record the date checked. Done 2026-11-13 (above).
2. Retire THREAT_MODEL A2 and A4 for CosmWasm deployments only. Done in `THREAT_MODEL.md`, on the basis of this re-read.
3. Keep A4 in force for any off-chain host that holds a `Ledger` directly.

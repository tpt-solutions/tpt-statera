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

Status: claims 1 and 3 were checked against search-result excerpts of the CosmWasm pages, not a full read. Claim 2 is
also checked in the simulator (see above).
Re-read the pages against the pinned `cosmwasm-std` 3.0.11 before relying on them.

What the chain does not give us, and the adapter does not supply:

- **Admin paths.** There is no admin message after instantiate. Any future admin path must be reviewed
  against this argument.
- **Instantiate trust.** The fee rate and initial balances are set by whoever instantiates the contract
  (THREAT_MODEL A3). The adapter cannot check that.
- **IBC and cross-contract calls.** Atomicity does not extend across chains, per the same sources. The
  adapter makes no cross-chain calls.

## Remaining path

1. Re-read the cited CosmWasm pages against cosmwasm-std 3.0.11 and record the date checked.
2. Retire THREAT_MODEL A2 and A4 for CosmWasm deployments only, after step 1.
3. Keep A4 in force for any off-chain host that holds a `Ledger` directly.

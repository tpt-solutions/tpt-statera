# Architecture Notes — tpt-statera

Decision records for `todo.md`. Each decision states its rationale, so a reviewer can reopen it
deliberately rather than by accident. Status markers: **Adopted** = in code; **Recommended** = chosen
and documented, not yet implemented.

## AD-1: Toolchain and lint policy (todo 1A.04) — Adopted

- **Edition 2021, MSRV 1.75** (`[workspace.package]`). Stable channel (`rust-toolchain.toml`).
- **`#![deny(unsafe_code)]`** in every crate, enforced by `[workspace.lints.rust]`.
- **Clippy `all` denied**, run with `-D warnings` in CI. `missing_docs` is a warning.
- **Arithmetic policy: deny-by-design, never wrap.** Intermediate values use `u128` or `i128`.
  Values are range-checked against the target `u64` before narrowing. Overflow of a `u64`
  addition is a `Deny`, not a panic or a wrap. Division is always Floor.
- **Not adopted: `clippy::arithmetic_side_effects`.** It would flag every operation in the
  wide-arithmetic core. Each narrowing cast instead carries a comment naming the check that
  proves it in range, and property tests compare results against wide arithmetic.

## AD-2: `no_std` posture (todo 1A.05) — Adopted for core

`tpt-statera-core` is `#![no_std]`, has zero dependencies, and performs no heap allocation.
`tpt-statera-runtime` uses `std` (it needs `Mutex`). That is acceptable because the runtime is a
host-side layer, not the code compiled into the chain artifact.

**Consequence:** the netting API writes into caller-supplied slices. The `wasm32` and `thumbv6m`
targets are listed in the toolchain notes. A CI `cargo check --target wasm32-unknown-unknown -p
tpt-statera-core` job is still to be added (see `.github/workflows/ci.yml`).

## AD-3: Dependency governance (todo 1A.07) — Adopted

- **Runtime dependencies of `tpt-statera-core`: none.** `tpt-math` (spec §4) is not used. The
  implemented operations need only `u128` and `i128`.
- **Proof toolchain:** `tpt-telos` is an external verifier binary, not a Cargo dependency. Version
  pinned in `SEMVER_CONTRACTS.md`. Resolution (1A.07d): **path fallback** to a sibling checkout.
- **`Cargo.lock` is committed**, so the workspace build is reproducible.
- **Bench harness is outside the workspace** (`bench/fm_netting`, its own `[workspace]`), so the
  sibling `tpt-telos` dependency does not change the product lockfile.

## AD-4: Rounding policy (todo 1D.05, 2A.02) — Adopted

Phase 1 and Phase 2 use **Floor only**: `fee = floor(amount × fee_rate_bps / 10_000)`.

- The protocol never charges more than the ideal rational fee, so the user is never overcharged.
- The remainder `dust = (amount × fee_rate_bps) mod 10_000` is never debited from the sender. It is
  allocated to a destination, as spec Principle 2 requires.
- `tpt-statera-policy` declares `Ceiling` and `HalfEven` with mandatory dust destinations
  (`SenderRetains`, `Treasury`, `Burn`), but `RoundingPolicy::new` **rejects** them. Both can
  overcharge the user by a sub-unit amount. They become permissible only after a sub-unit refund
  accumulator is designed.

## AD-5: Restated conservative-denial clause (todo 1D.02) — Adopted (recommendation)

**Problem.** Spec §6.1 and Appendix A state `ideal Deny ⟹ realized Deny`. Under Floor rounding this
is false. Example: `amount = 1`, `fee_rate_bps = 5_000`, `sender_balance = 1`. The ideal total is
1.5, so the ideal model denies. The realized fee is `floor(0.5) = 0`, so the realized total is 1,
and the realized model allows. The sender ends at 0 and nothing overdraws, so the safety property
holds even though the clause fails. A second cause is treasury overflow: the ideal treasury credit
can exceed `u64::MAX` while the floor credit does not.

**Decision.** Option 2 from the earlier analysis, in this exact form:

> `ideal Deny ⟹ realized Deny`, **or** the ideal shortfall is below one unit, where the shortfall
> is `amount + numer/10_000 − sender` for the solvency cause, and `treasury + numer/10_000 − MAX`
> for the overflow cause.

**Why option 2 and not option 1.** Option 1 ("no overdraft under exact arithmetic") is the safety
property alone, and it drops the link to the ideal model that §6.1 exists to make. Option 2 keeps
that link and states precisely what the Floor rounding is allowed to change: less than one unit,
on either cause.

**Evidence.**
- `crates/tpt-statera-core/tests/differential.rs::restated_conservative_denial_clause_holds` checks
  both causes over 200 000 generated cases, and requires both gap types to occur.
- `proofs/transfer_validator.telos` states the solvency and overflow shortfalls in the
  specification but **does not** prove them. The solver's disjunction handling is unsound in this
  version (see AD-6), so the clause is held by the Rust test.

**Spec follow-up.** Spec §6.1 gets a revision-history row for this decision (done in this pass),
and the Appendix A comment is to be updated by the spec owner.

## AD-6: Scope of the formal models — Adopted, with a solver defect recorded

### What the solver can and cannot do

- **Linear is fine.** Products with a constant (`x * 10000`) are linear.
- **Bilinear is not.** A product of two variables, or a division by a variable, is out of reach for
  the built-in Fourier–Motzkin solver. `--solver z3` would handle it but is not available here.
- **Floor can be encoded linearly.** The Floor fee is characterized by
  `fee·10000 ≤ numer < fee·10000 + 10000`, with `numer = amount·bps` supplied as a parameter. The
  characterization is linear in `(fee, numer)`, so the solver can use it. The product `amount·bps`
  is still an assumption, not a derivation. This is the encoding used in `transfer_validator.telos`.
- **Vesting cannot be encoded this way.** `floor(elapsed·alloc / total)` divides by a variable, so it
  needs a bilinear constraint `v·total ≤ numer`. Blocked.

### A solver defect found by negative control

Disjunctive `ensures` clauses (`A || B`, `A ==> B`) are not reliable in this telos version. In a
negative control, the overflow guard was removed, and the model still reported **PASS**. Each
disjunctive obligation is reported per branch, and the group verdict can pass when a real path
fails. The disjunctive clauses were therefore removed from the proof, and the behavior they describe
is covered by the Rust tests.

A second defect came from the composition proof. When one variable is assigned in two `mutate`
blocks, the verifier's result does not follow the program: both negative controls passed. The
composition was rewritten with each balance assigned once in a single block, and then both controls
failed as they should. Rule for every telos model in this repo: **assign each state variable at most
once per function**, and check every model with a negative control.

**Consequence.** The telos files prove only the conjunctive clauses in single-assignment form, and
each has a negative control that fails as it should. Anything stated as a disjunction, or any
variable assigned twice, is **not** a machine-checked result. Both are upstream issues to report to
the `tpt-telos` owners.

## AD-7: Serialized validate-and-commit (todo 2B.05) — Adopted (in-process)

`tpt-statera-runtime::Ledger::submit` locks the accounts it touches in ascending id order, takes a
versioned snapshot, validates, re-checks the versions, and commits. A fixed lock order prevents
deadlock. Balances are private, so `submit` is the only write path.

**Limit.** This is in-process only. A host that keeps a `Ledger` can skip `submit` (spec §8,
Assumption 4). Cross-process serialization needs a database, chain nonce, or actor. The
concurrency test (8 threads, one sender) checks conservation and no overdraft.

## AD-8: First host target for the runtime (todo 2B.01) — Recommended

**Recommendation: CosmWasm first**, Cosmos SDK module second, generic Rust–Go FFI last.

- The core is `no_std` with zero dependencies, which is the shape CosmWasm needs (AD-2). A Cosmos
  SDK module would need a Go bridge, which adds an FFI surface to audit.
- CosmWasm's contract model gives a single serialized execution per message, which is the
  per-account serialization that the threat model requires (T5), without a bespoke lock.
- The interception ABI is therefore: the contract's `execute` entry point decodes a
  `TransferProposal` and calls `validate_transfer` on the state it reads. The host-level
  bypass argument (2B.04) still needs writing. The contract cannot be bypassed on-chain, but only
  if the state layer is itself trusted.

Not built yet. The CosmWasm adapter is the next runtime task.

## AD-9: Hardware reference (todo 1E.01) — Adopted (recommendation)

**Recommendation: freeze the current workstation as the reference hardware, with its settings
recorded.** A dedicated machine is preferable for third-party reproduction. Until one exists, this
host is the reference, and the record in `bench/REFERENCE_HARDWARE.md` is authoritative. See that
file for the settings and the limits that still apply.

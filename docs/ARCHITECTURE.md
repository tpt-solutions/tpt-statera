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

A third limit, tested in this pass: telos bodies cannot call a function as a statement. Calls
are expressions that expand only inside specifications (`docs/LANGUAGE.md`, "Function & method calls").
A per-outcome decomposition (one function per Allow or Deny reason, with the reason as a requires) was
therefore not expressible. Telos ensures apply to every path, so the upper rounding bound and the
overflow denial cannot be stated per branch without disjunction. They stay Rust-tested.

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

---

## Update: approved follow-up work (third pass)

### Correction to AD-6: the defects are in IR lowering, not in the solvers

AD-6 called the disjunction and repeated-assignment failures "solver" defects. Re-checking with both
engines shows otherwise. The broken disjunction model passes under Fourier–Motzkin **and** under Z3.
The repeated-assignment control passes under both as well. So the fault is in how `tpt-telos` lowers
`ensures` and `mutate` into constraints (`crates/tpt-telos-ir`), which both engines share. Upstream
repros are in `proofs/upstream-repros/`.

### Z3 (approved, partly done)

- **Z3 is installed** from the official `z3-5.4.1-x64-win.zip` into `<local-tools-dir>/z3-5.4.1-x64-win`.
  It is not installed system-wide, because Chocolatey needs administrator rights.
- **The build needed three things:** `libclang` for bindgen (from the PyPI wheel, user install), the Z3
  header and import library, and a one-file port of `tpt-telos-verifier/src/z3_solver.rs` to the `z3` 0.12
  API. The port is saved as `proofs/upstream-repros/tpt-telos-z3-api-0.12.patch`. The sibling repo was edited
  in place to build it, so that patch needs upstreaming.
- **Agreement on linear proofs:** `transfer_validator`, `composition_two_transfers` and `netting_10party`
  verify the same under Fourier–Motzkin and Z3.
- **Nonlinear is still blocked, and Z3 does not unblock it by itself.** The front end rejects a product of
  two variables (`non-linear multiplication is not supported in constraints`), so the Z3 backend never sees
  it. Telos has an interval-bounding fallback for products, but it is an **approximation**, which the
  transcript flags. It is not an exact derivation, so this project does not use it.
- **What would unblock it:** extend the IR so that a constraint can hold a monomial (a product of
  variables), let the Z3 backend encode it, and make Fourier–Motzkin refuse monomials. That is a design
  change in `tpt-telos-ir`, `extract.rs` and the solver. It is **not** done, and it needs approval before
  anyone starts it.

### Cross-process serialization (approved): single writer

`tpt-statera-runtime::service` runs one writer thread that owns the `Ledger` and is the only writer.
In-process clients use `WriterService`. Cross-process clients use a line protocol over TCP (`serve_tcp`
and `TcpClient`). It was verified across separate processes with the `ledger_server` and `ledger_client`
examples: committed, denied, unknown-account and self-transfer replies all behave as expected.

Limits, stated in the module docs: there is **no authentication, TLS or authorization**. Anyone who can
reach the port can move funds. Bind to loopback or a private network only, and add access control at the
transport layer before any wider use. The single writer is also a single point of failure and a throughput
bottleneck.

### CosmWasm adapter (approved, built): AD-8 implemented

`crates/tpt-statera-cosmwasm` is built and tested (5 unit tests). The contract is pinned to
`cosmwasm-std` **3.0.11**, published 2026-10-08, more than two weeks before the pin. The `wasm32` release
build (371 KB) exports `instantiate`, `execute`, `query`, `allocate`, `deallocate` and `interface_version_8`.
It imports only the standard host functions.

Two build decisions were needed, and both are recorded in the code and in `.cargo/config.toml`:

1. **Iterator feature off.** The contract never iterates storage on-chain. Leaving the feature on pulls in
   the `db_next` import, which the link step cannot resolve.
2. **`--allow-undefined` for the `wasm32` target only.** The host functions are plain `extern "C"` imports,
   and this toolchain's `rust-lld` rejects them as undefined unless this flag is set. It is scoped to the
   `wasm32-unknown-unknown` target, so host builds are unaffected.

The only state-changing path is `Transfer`. It writes three balances on `Allow` and returns an error on
`Deny`, so CosmWasm reverts the message.

**Not verified here:** `cosmwasm-check`, and a run on a live chain or a `cw-multi-test` harness. Both are
open items for the next pass.

---

## Update: monomials approved and implemented (fourth pass)

The earlier "what would unblock it" design is now implemented. The Z3 gate was already in telos
(`--solver z3`, the `z3` cargo feature, and `solver_backend()`). It was not enough by itself, because
the IR rejected a product before any solver ran. The change in the sibling `tpt-telos` repo, saved as
`proofs/upstream-repros/tpt-telos-monomials.patch`:

- **IR.** `Linear` gains `products: Vec<(Linear, Linear, i64)>`, meaning `k * a * b`. Constructors,
  add, sub, neg and scale carry it. `Linear::product(a, b)` and `has_products()` are new.
- **Lowering.** A product of two non-constant expressions is lowered to a monomial only when exact mode
  is on. `set_exact_nonlinear(true)` is set by `--solver z3`. The default path is unchanged. The
  interval-approximation branch is still used by default, and it is skipped in exact mode.
- **Bounds.** A constraint with a monomial does not give a variable bound.
- **Z3.** `linear_to_z3` encodes monomials exactly. Model variables are collected inside products.
- **Fourier-Motzkin.** Every public entry point refuses constraints with monomials and fails closed
  (`entails` returns false, `model` returns none).

Verified this pass:

- `floor_fee_exact.telos` and `vesting_exact.telos` pass under `--solver z3`, and each has a negative
  control that fails.
- The default solver still refuses both with the old error.
- The linear proofs agree under both solvers. The telos IR and verifier tests (32 + 29 + 9 + 12 + 8 + 11 + the
  rest) pass.

What is still blocked: the upper rounding bound and overflow denial, which need disjunctive ensures. The
lowering defect in AD-6 is still open. Monomials do not fix it; that is the next item.


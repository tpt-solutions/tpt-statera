# tpt-statera — Master Task Checklist

> Formally Verified Financial Invariant & Ledger Engine
> Source of truth: `spec.txt` (Draft v1, 2026-08-20) · Owner: **TPT Solutions** · License: **MIT OR Apache-2.0**
> Structure follows the spec §7 compliance roadmap (Phases 1–4 as top level); crate workstreams are slotted inside each phase.

## Conventions

- `- [ ]` open · `- [x]` done. Stable item IDs (`1A.01`) for referencing in commits/PRs/issues.
- **AC:** = acceptance criteria / definition-of-done, attached to tasks tied to Phase 1 exit criteria or audits.
- **EC1–EC5** = Phase 1 Exit Criteria (spec §9).

## Traceability — Phase 1 exit criteria → tasks

| EC | Requirement (spec §9) | Covering tasks |
| :-- | :--- | :--- |
| EC1 | `validate_transfer` passes ideal-layer **and** realization-layer proofs, incl. Strict Conservation of Value + No False Allows (overdraft) postconditions | 1C.01–1C.03, 1D.01–1D.03 |
| EC2 | FM-elimination scalability benchmark on multi-party settlement (synthetic 10-party netting); median < 10 ms, p99 < 50 ms on documented reference hardware | 1B.08, 1E.01–1E.04 |
| EC3 | First-draft Proof Certificate generated and mapped toward SOC2/MiCA objectives, explicitly labeled illustrative | 1F.01–1F.04 |
| EC4 | Two-layer soundness demonstrated on ≥ 2 functions beyond `validate_transfer` (linear vesting + multi-party netting); netting proof explicitly addresses constraint-orthogonality assumption | 1B.07–1B.08, 1C.04–1C.05, 1D.04–1D.05 |
| EC5 | Written threat-model document reviewed and signed off | 1G.01–1G.03 |

---

## Phase 1 — Verified Stateful Transfer Validator + First-Draft Certificate

### 1A. Foundation, Licensing & Workspace

- [x] `1A.01` Initialize repository: `.gitignore`, `README.md` with §1 positioning + prominent status banner ("Phase 1 scope only; §2 guarantees are target architecture until runtime + multi-account composition are verified").
- [x] `1A.02` Scaffold Cargo workspace `tpt-statera`: real members `tpt-statera-core`, `tpt-statera-proof`; placeholder stub crates `tpt-statera-policy`, `tpt-statera-runtime` (compile-empty, marked out of scope).
- [x] `1A.03` Dual licensing: add full `LICENSE-MIT` and `LICENSE-APACHE` texts; set `license = "MIT OR Apache-2.0"` in every manifest; add `// SPDX-License-Identifier: MIT OR Apache-2.0` headers with **TPT Solutions** copyright to all source files.
- [x] `1A.04` Toolchain decisions: Rust edition, MSRV, `rustfmt.toml`, clippy profile (deny warnings in core), `#![deny(unsafe_code)]` in core, documented arithmetic-overflow policy (deny-by-design, never wrap).
- [x] `1A.05` Decide `no_std` posture for `tpt-statera-core` (runtime crate will target CosmWasm / Cosmos SDK); record decision + rationale in an architecture note.
- [ ] `1A.06` CI pipeline: fmt check, clippy, unit + property tests, doc build (warnings as errors), coverage; separate nightly job for proof runs + benchmarks.
- [ ] `1A.07` Dependency governance (TPT ecosystem):
  - [x] `1A.07a` Pin exact versions of `tpt-math`, `tpt-formal`, `tpt-telos`; commit `Cargo.lock`. (The workspace has no Cargo dependency on `tpt-math` or `tpt-formal`; `tpt-telos` is pinned by commit SHA `bfa9637` in `nightly.yml`; `Cargo.lock` is committed.)
  - [x] `1A.07b` Write `SEMVER_CONTRACTS.md`: which upstream APIs/types we compile against and our bump policy.
  - [ ] `1A.07c` Upstream regression smoke tests in CI that fail loudly on behavioral drift (solver answers, math kernels).
  - [x] `1A.07d` Decide registry vs. vendored/path fallback strategy for TPT-internal crates.

### 1B. `tpt-statera-core` Implementation (spec Appendix A)

- [x] `1B.01` Types per Appendix A: `TransferProposal { amount, fee_rate_bps }`, `SystemState { sender_balance, recipient_balance, treasury_balance }`, `ValidationOutcome::Allow { .., dust_retained_by_sender } | Deny` — use `dust_retained_by_sender`, **never** `dust_refunded_to_sender` (§6.1 renaming rationale).
- [x] `1B.02` Implement `validate_transfer` steps 0–6: fail-closed `fee_rate_bps > 10_000` guard; solvency check; u128 floor fee + captured `dust` excluded from `total_deduction`; u128-computed recipient/treasury sums checked against `u64::MAX` **before** any u64 addition/cast.
- [x] `1B.03` Unit-test edge table: zero amount; zero fee; 9999 / 10000 bps; 10001 bps → Deny; sender exactly solvent vs. short by 1 unit; recipient at `u64::MAX`; treasury at `u64::MAX`; dust = 0 and dust > 0 partitions.
- [x] `1B.04` Property tests (arbitrary valid inputs): conservation `post_sum == pre_sum` on Allow; directionality (sender decreases only, recipient/treasury increase only); Deny paths never panic or wrap (checked in release mode too).
- [x] `1B.05` Differential oracle tests vs. `symbolic_evaluate_ideal` reference model: `|realized_fee − ideal_fee| ≤ 1.0` and ideal-Deny ⟹ realized-Deny.
- [x] `1B.06` API docs carrying the scope notes verbatim: single-proposal/single-snapshot non-atomicity warning (§8.2); comments marking `fee_rate_bps ≤ 10000` and `sender_balance ≥ amount` as deliberate trust-boundary `requires` clauses so reviewers don't convert them to `ensures` (§6.1 final row).
- [x] `1B.07` Demo function #1: **linear vesting calculator** — `claimable ≤ (elapsed/total) × total_allocation`, bounded by fixed-point precision, truncation dust explicitly allocated (feeds EC4).
- [x] `1B.08` Demo function #2: **multi-party settlement netting** over N parties — Σ of all party state changes == 0; ship a reusable synthetic 10-party fixture shared with the benchmark harness (feeds EC2 + EC4).

### 1C. Ideal-Layer Verification (QF_LRA exact rationals, via `tpt-telos`)

- [x] `1C.01` Author ideal-layer contracts for `validate_transfer`: `requires fee_rate_bps ≤ 10000`, `requires sender_balance ≥ amount` (annotated as documented trust boundaries, §8.3); `ensures` no-overdraft triple; `ensures` strict conservation `post_sum == pre_sum`. (Done in `proofs/ideal_transfer_validator.telos`: exact fee in scaled integers, PASS under Z3, negative control `proofs/negative/ideal_creates_value.telos` FAILs. The `sender_balance ≥ amount` trust boundary is the Allow guard, not a `requires`.)
  - **AC (part of EC1):** telos discharges both `ensures` clauses universally (FM elimination completes with nothing left unchecked); no added assumptions beyond the §8 list.
- [x] `1C.02` Enforce the independent-clause-scope rule: no contract clause references another clause's `let` bindings; overflow clause recomputes `max_possible_fee` inline from inputs only (§6.1 rows 3 & 7).
- [x] `1C.03` Edge-case proof sweep: zero-value transfers, u64 min/max bounds, fee extremes (0, 9999, 10000 bps) — confirm covered by the universal proof, not just by tests; archive solver transcripts as evidence artifacts.
- [x] `1C.04` Ideal-layer contracts for the vesting calculator (temporal/proportional invariant, §2 tokenomics row). (Done in `proofs/ideal_vesting.telos`: exact claim `elapsed·alloc` in scaled units, PASS under Z3; negative control `proofs/negative/ideal_vesting_creates_value.telos` FAILs.)
- [x] `1C.05` Ideal-layer contracts for netting: Σ party deltas == 0; explicitly state the constraint-orthogonality assumption and justify its validity for the 10-party fixture.
  - **AC (part of EC4):** orthogonality assumption appears in the proof artifact with a written justification.

### 1D. Realization-Layer Soundness (u64 integers, bounded rounding)

- [ ] `1D.01` Bounded-rounding `ensures`: `abs(rounded_fee − ideal_fee) ≤ 1.0`, with Floor semantics (realized fee ≤ ideal, error ∈ [0, 1)).
- [x] `1D.02` Conservative-denial `ensures`: ideal Deny ⟹ realized Deny (**No False Allows / overdraft prevention**).
- [ ] `1D.03` Additive-overflow `ensures` in input-pure form: `(recipient_balance + amount > u64::MAX ∨ treasury_balance + max_possible_fee > u64::MAX) ⟹ Deny`, where `max_possible_fee = floor(amount × fee_rate_bps / 10000)` is recomputed inline (§6.1 rows 6–7).
  - **AC (completes EC1):** ideal (1C) + realization (1D) proofs for `validate_transfer` both pass in CI on every push; failures block merge.
- [x] `1D.04` Two-layer soundness demonstrated on **both** extra functions (vesting + netting): paired ideal + realization contracts with per-function ideal→realization gap bounds. (Vesting: `ideal_vesting.telos` + `vesting_exact.telos`, gap in [0,1) raw unit. Netting: exact linear arithmetic with no rounding, so the ideal and realized forms are the same and the gap is identically 0; `netting_10party.telos` is its only model. Gap bounds are written up in `proofs/README.md`.)
  - **AC (EC4):** both functions verified in both layers; write-up documents the gap bound per function.
- [x] `1D.05` Document rounding-mode policy: Phase 1 = **Floor only** (protocol never overcharges); `Ceiling`/`HalfEven` deferred to `tpt-statera-policy` with mandatory dust-destination routing (§5.2, Principle 2).

### 1E. FM-Elimination Scalability Benchmark (EC2)

- [x] `1E.01` Define and freeze the **reference hardware spec** (replaces "spec TBD"): CPU model, cores, RAM, OS, solver build flags, RNG seeding; commit as `bench/REFERENCE_HARDWARE.md`.
- [x] `1E.02` Build harness: drive FM elimination over the 10-party netting constraint set; measure per-call latency; statistical protocol (run count, warm-up, median + p99 reporting).
- [x] `1E.03` Tractability gate: median < **10 ms** AND p99 < **50 ms** per call.
  - **AC (EC2):** passing run archived with raw latency data + hardware ID; result reproducible by a third party from the committed manifest.
- [ ] `1E.04` Nightly CI benchmark regression tracking with alert threshold (e.g., > 20% median degradation auto-opens an issue).

### 1F. First-Draft Proof Certificate (`tpt-statera-proof`, EC3)

- [x] `1F.01` Finalize Proof Certificate format: versioned, machine-readable schema mapping verification records → control objectives; consumes `tpt-formal` audit-trail output; schema doc published. (**Decision:** `tpt-formal` has no audit-trail output, so the certificate consumes `telos verify --json` instead. This replaces the `tpt-formal` clause. Recorded in `docs/CERTIFICATE_SCHEMA.md`, which is the published schema doc, `0.1-draft`.)
- [ ] `1F.02` Generate first-draft certificate covering **`validate_transfer` only**.
  - **AC (EC3):** certificate regenerates deterministically in CI; every claim carries a proof ID traceable to a stored solver transcript (1C/1D artifacts).
- [x] `1F.03` Embed the §7 caveat verbatim: illustrative artifact; not a compliance claim; excludes concurrent-access and multi-party netting scenarios; must not be presented to customers/auditors as a completed regulatory mapping until Phase 2 + Phase 3.
- [x] `1F.04` Stub control-mapping entries for SOC2 CC6.1 and MiCA Article 51, explicitly marked "mapping pending Phase 2".

### 1G. Threat Model & Responsible-Claims Documentation (EC5)

- [x] `1G.01` Promote spec §8 into standalone `THREAT_MODEL.md` (successor doc): 4 assumptions, serialized-validation caller obligation, trusted-config boundary, no-enforcement-bypass-yet statement.
- [x] `1G.02` Claims-guardrail template: approved external wording ensuring "conservation of value" is never cited without its scope (single-call, serialized execution, Phase 1 boundary).
- [ ] `1G.03` Review cycle: internal reviewer(s) + sign-off block (names, roles, dates) recorded in the doc.
  - **AC (EC5):** sign-off recorded; doc version-tagged; linked from `README.md` and referenced by the Proof Certificate.

### 1H. Phase 1 Exit Review

- [ ] `1H.01` Assemble evidence pack mapping EC1–EC5 → concrete artifacts (proof transcripts, benchmark report, certificate, threat-model sign-off).
  - **AC:** all five criteria evidenced and checked; any gap reopens its source workstream before exit.
- [ ] `1H.02` Tag release (e.g., `tpt-statera-core v0.1.0`) + `CHANGELOG.md` entry whose claims stay inside the §7 caveat.
- [ ] `1H.03` Retrospective: fold any contract/code divergences discovered during verification back into `spec.txt` §6.1 revision history.

---

## Phase 2 — Policy & Runtime Crates + SOC2/MiCA Compliance Mapping

### 2A. `tpt-statera-policy` Crate

- [x] `2A.01` Specify policy model (`docs/POLICY_MODEL.md`, decisions D1–D5 taken as recommended; rate limits and proportional distribution deferred to the backlog): fee structures (tiered, split-recipient), rate limits, proportional distribution rules.
- [x] `2A.02` Rounding-mode enum (`Floor`, `Ceiling`, `HalfEven`) where **every** mode declares an explicit dust destination (sender refund / treasury / burn) — no silent vanishing (Principle 2).
- [x] `2A.03` Implement policies + property tests mirroring the 1B.03–1B.05 methodology.
- [ ] `2A.04` Telos proofs: policy definitions internally consistent; provably cannot produce negative balances or overflow (crate-map target column, §4).
- [x] `2A.05` Re-run FM benchmark against policy-augmented constraint sets; confirm EC2 latency gates still hold; update `bench/` reports.

### 2B. `tpt-statera-runtime` Crate (FFI Bridge)

- [x] `2B.01` Select host integration targets (CosmWasm, Cosmos SDK module, generic Rust⇄Go FFI) and design the interception ABI.
- [x] `2B.02` Interception layer: every state transition routes through validated core/policy logic before commit.
- [x] `2B.03` Snapshot integrity: verify host-environment state matches the verified `SystemState` snapshot at call time (anti-TOCTOU argument).
- [x] `2B.04` Bypass-resistance: formal argument that the FFI boundary cannot be skipped (crate-map target column, §4).
- [x] `2B.05` Close the §8.2 concurrency gap: enforce/verify serialized validation-and-commit per account (mutex / actor / blockchain nonce) at the runtime layer; update `THREAT_MODEL.md` to retire Assumption 2's caller obligation.

### 2C. Compliance Mapping (SOC2 / MiCA / Financial Audit)

- [x] `2C.01` Control-selection matrix: SOC2 CC6.1 (Logical Access Security), MiCA Article 51 (Asset safeguarding), plus alternates legal counsel identifies ← certificate fields.
- [ ] `2C.02` Legal/compliance review of all external claims language; remove "illustrative" labels only where the delivered phases earn it.
- [x] `2C.03` Multi-account composition verification: extend single-transition proofs to composed/portfolio-level conservation (required for §2 guarantees to apply).
- [ ] `2C.04` Produce auditor-ready evidence pack v1 in the format auditors actually consume.
  - **AC (§7 Phase 2 row):** abstract proofs converted into mapped, traceable compliance evidence reviewed by a compliance stakeholder.

### 2D. Release Engineering & Hardening

- [ ] `2D.01` Publish crates (core, proof, policy, runtime) with dual-license metadata; pre-publish `cargo package` dry-runs; evaluate + integrate dependency-vulnerability scanning.
- [ ] `2D.02` Disclosure policy (`SECURITY.md` removed at owner request) with TPT Solutions security contact and response SLAs.
- [ ] `2D.03` Docs site / API reference with worked examples per §2 industry vertical (DeFi, payments, tokenomics, supply-chain finance).
- [ ] `2D.04` Refresh upstream pins + regression suite for any new `tpt-telos` capabilities consumed by policy/runtime proofs.

---

## Phase 3 — Independent External Audit

- [ ] `3.01` Shortlist third-party financial security firms with formal-methods + financial practice (spec examples: Trail of Bits, Halborn); issue RFP scoped to the **Conservation of Value** and **Explicit Dust** rounding arguments (§7 Phase 3 row).
- [ ] `3.02` Select firm; execute SOW covering access to specs, solver transcripts, threat model, benchmark harness; agree success criteria and remediation windows.
- [ ] `3.03` Audit-readiness package: frozen repo snapshot, reproduction instructions for proofs/benchmarks/certificates, known-limitations list lifted from `THREAT_MODEL.md`.
- [ ] `3.04` Audit support window: findings triage, proof walkthroughs, question-turnaround commitments.
- [ ] `3.05` Remediation: resolve or formally justify each finding; re-run affected proofs + benchmarks; add a regression test per code-level finding.
  - **AC:** all critical/high findings closed with re-verified proofs; medium/low findings triaged with documented rationale accepted by the auditor.
- [ ] `3.06` Auditor re-review and final attestation letter; publish the report **with scope caveats intact** (serialized execution, phase boundaries).
  - **AC (§7 Phase 3 row):** external confirmation that the zero-value-creation claim holds under external scrutiny.

## Phase 4 — Reference Submission Package

- [x] `4.01` Define package contents: template audit submission assembled around a runnable customer-style deployment built on the Phase 1 transfer validator (§7 Phase 4 row).
- [x] `4.02` Build the reference application end-to-end: configure system state → submit proposals → generate certificates → export the audit bundle.
- [x] `4.03` Walkthrough guide: how customers adapt it (custom fee structures via `tpt-statera-policy`, runtime integration points, evidence regeneration).
- [ ] `4.04` Compliance packaging checklist informed by Phase 3 audit experience (what auditors requested, how it was satisfied).
- [ ] `4.05` Legal review of package claims; align caveats with post-audit reality (Phase 2 mappings delivered, Phase 3 audit complete).
- [ ] `4.06` Publish package v1; establish feedback channel; assign maintenance ownership.
  - **AC (§7 Phase 4 row):** package converts "auditable" from an abstract claim into a concrete, reusable artifact a customer can execute unmodified.
- [x] `4.07` Backlog refresh for the next planning cycle: revisit deferred scope (non-linear vesting, additional rounding modes, multi-currency ledgers, further §2 verticals).

---

*End of checklist. Keep IDs stable when adding items — append new ones rather than renumbering.*

---

---

## Status notes (second pass)

Done in the second pass (see `docs/ARCHITECTURE.md`, `proofs/README.md`, `CHANGELOG.md`):

- Restated 1D.02 (AD-5), tested for both gap causes. Floor-fee encoding in `transfer_validator.telos`.
- Two solver defects found by negative control and recorded (AD-6): disjunctions and repeated assignments.
- FM netting benchmark run (5 000 calls, raw CSV). Gate passes on the frozen provisional reference host.
- Policy crate (Floor only), runtime `Ledger` (in-process serialization, version re-check, 8-thread test).
- Worked examples and reference settlement bundle. Composition proof (two steps, net effect).
- CI extended with `wasm32` and `thumbv7m` `no_std` checks. Not run remotely.

Still open, and why:

- `1A.06`, `1A.07a`, `1A.07c`, `1E.04`: CI and nightly jobs are written but not run remotely. Nightly
  proofs are disabled until the `tpt-telos` checkout location is decided.
- `1C.01`, `1C.03`, `1C.04`, `1D.01`, `1D.03`, `1D.04`: the solver cannot prove disjunctive clauses
  (upper rounding bound, overflow denial) or division by a variable (vesting). Rust tests cover the
  first two. Blocked on upstream (`docs/BACKLOG.md` items 1–3).
- `1F.01`: schema documented (`docs/CERTIFICATE_SCHEMA.md`), but `tpt-formal` output is not consumed.
- `1F.02`: certificate generated and deterministic locally. CI determinism check not wired.
- `1G.03`, `1H.*`: threat-model sign-off and exit review need named human reviewers. `1H.02` tag needs the
  owner's approval. `1H.03` spec Appendix A comment still to update.
- `2A.01`, `2A.03`–`2A.05`: fee tiers, rate limits, proportional distribution, telos proofs for policy,
  and the FM re-run on policy-augmented sets are not built.
- `2B.03`–`2B.05`: host-environment snapshot check and the bypass argument need the CosmWasm adapter.
  Cross-process serialization is not built.
- `2C.02`, `2C.04`, `4.05`: legal and compliance review, and an auditor-consumed evidence format, need people.
- `2C.03`: two-step composition proven. N-step is by induction, not checked.
- `2D.01`: `cargo package --list` works. Publishing is blocked by `publish = false`, which is an owner decision.
- `2D.02`: security policy removed at owner request; not reinstated.
- `2D.03`: examples exist for payments, vesting and netting. No DeFi example, and no docs site.
- `3.01`–`3.06`: RFP scope drafted (`docs/RFP_SCOPE.md`). No firm chosen, no SOW, no audit.
- `4.02`: the reference application copies the certificate and writes the bundle. It does not run the
  solver itself (that stays a separate `telos` step).
- `4.04`, `4.06`: compliance checklist is not informed by an audit yet. Publishing needs the owner.

---

## Status notes (third pass)

- `SECURITY.md` removed at the owner's request. References in README, CHANGELOG and this file updated.
  `2D.02` stays open; the policy is not reinstated.
- Tried a dispatcher proof (one function per outcome) to avoid disjunctions. Telos does not allow a
  function call as a statement in a body, only inside specifications. Removed; the limit is recorded in
  `docs/ARCHITECTURE.md` AD-6. The upper rounding bound and overflow denial remain Rust-tested only.
- Spec Appendix A comment updated to the restated denial clause (AD-5). Done.
- `2D.03`: DeFi pool-payout example added (`defi_pool_payouts`). It is not an AMM; pricing is out of scope.
- `1E.04`: `bench/compare_bench.py` compares the median of several runs against the committed baseline
  and fails above a 20% median degradation. Measured: single 1 000-call runs gave +32%, and three
  5 000-call runs gave a median-of-runs of −3.6%. One run is not enough, so the script needs several.
  It is not wired into CI. The alert does not open an issue automatically.
- `2B.04`: bypass argument written (`docs/BYPASS_ARGUMENT.md`), with the conditions it depends on and
  what it does not prevent. Host-level argument still needs the CosmWasm adapter.

---

## Status notes (fourth pass: Z3, cross-process, CosmWasm)

- **Z3 (approved):** installed from the official zip into `<local-tools-dir>` (not system-wide). Telos
  builds with `--features z3` after a one-file API port (`proofs/upstream-repros/tpt-telos-z3-api-0.12.patch`,
  applied to the sibling repo in place, needs upstreaming). Linear proofs agree under both solvers.
- **Correction:** the disjunction and repeated-assignment defects are in IR lowering, not in the solvers.
  Both solvers pass the broken models. Repros are in `proofs/upstream-repros/`.
- **Nonlinear (`1C.01` derivation, `1C.04`, `1D.01` upper half, `1D.03`):** still blocked. The IR rejects
  products of variables before the solver runs. Lifting that needs an approved IR change (monomials). Not started.
- **Cross-process (approved):** `tpt-statera-runtime::service`, a single writer with a TCP line protocol.
  Verified across separate processes. No authentication or TLS, so loopback or a private network only.
  `2B.05` is checked off for this design. THREAT_MODEL A2 is retired only for CosmWasm deployments, after the
  source re-read in the next item.
- **CosmWasm (approved):** `tpt-statera-cosmwasm`, pinned to `cosmwasm-std` 3.0.11 (published 2026-10-08).
  5 unit tests pass. The `wasm32` build (371 KB) exports the standard entry points. Two build decisions:
  iterator feature off, and `--allow-undefined` scoped to `wasm32` in `.cargo/config.toml`.
- **`2B.04`:** bypass argument extended to CosmWasm. Its three claims are cited to CosmWasm pages but were
  checked against search excerpts only. Re-read before relying on them.
- **Still open:** a live-chain or `cw-multi-test` run, `cosmwasm-check`, the source re-read, the nonlinear IR change,
  and the upstream fix for the lowering defects.
- Workspace: 56 tests pass in debug and release. Clippy with `-D warnings` is clean. `no_std` core builds for
  `thumbv7m` and `wasm32`.

---

## Status notes (fifth pass: monomials)

- **Approved and implemented:** monomials in the telos IR (`Linear::products`), exact lowering under
  `--solver z3`, Z3 encoding, Fourier-Motzkin refusal. Saved as `proofs/upstream-repros/tpt-telos-monomials.patch`.
  The sibling repo has these changes uncommitted.
- **Telos already had a Z3 gate** (`--solver z3`, the `z3` feature, `solver_backend()`). The gate was not
  enough alone: the IR rejected products before the solver ran. Both are now fixed.
- **Nonlinear proofs now verify under `--solver z3`:** `proofs/floor_fee_exact.telos` (conservation, no
  overdraft, directionality, never-overcharge with the exact product) and `proofs/vesting_exact.telos` (conservation,
  bounds on the claim). Each has a negative control that fails.
- **Still open:** the upper rounding bound and overflow denial (disjunctive ensures). The defect is in IR lowering
  of disjunctions, and monomials do not fix it. The certificate is not regenerated for the z3-only models, since its
  generator runs the default solver.
- **Not done:** the telos change has no upstream PR; the z3 0.12 port needs upstreaming too.

---

## Status notes (sixth pass: disjunction fix, full model)

- **Telos disjunction defect fixed** (tpt-telos `60d44cf`). The verifier counted each constraint of a DNF
  branch as an alternative. A branch is a conjunction, so it now holds only when every check does. The
  overflow-denial negative control now FAILs under both solvers. Regression tests added. Note: these
  tests were not run against the old commit, so their failing-before behaviour is reasoned, not observed.
- **Full model verifies under Z3:** `proofs/transfer_validator_full.telos` proves every clause of
  `validate_transfer` with the exact Floor product. Its negative controls FAIL.
- **Certificate** is regenerated under Z3 from the full model: 16 claims, all PASS, deterministic.
  The generator defaults to `--solver z3`.
- **Restated denial (1D.02):** proven under Z3 in the full model, in addition to the Rust test.
- **Still open:** repeated-assignment handling in telos (repro in `proofs/upstream-repros/`). The ideal layer
  as a separate exact-rational model. Vesting over the full schedule.
- **Commits:** statera and tpt-telos work was committed to `master` in stages (see `git log`).

---

## Status notes (seventh pass: telos soundness fixes)

- **Disjunction grouping** fixed earlier (`60d44cf`). The full model verifies under Z3, and its negative controls FAIL.
- **Expression statements** were lowered as assignments of `0`. A false claim `tokens == 0` passed. Fixed with
  `Stmt::Expr` (`980a92f`), with a regression test.
- **Repeated field assignment** passed vacuously. It is now refused with an error (`980a92f`), with a regression test.
- **Certificate** regenerated with the fixed binary: byte-identical, 16 claims, all PASS.
- **Open:** generated code does not return a function's trailing value (codegen, backlog item 13). Verification is
  unaffected.
- **Commits:** telos `60d44cf`, `980a92f`; statera follow-up commits in `git log`.

---

## Status notes (eighth pass: verification re-run, CI gate, pinning)

Verified locally on this machine, not on GitHub:

- **Proof runner:** `telos.exe` built from tpt-telos `bfa9637` with `--features z3` (the earlier binary in
  `.cargo/bin` predates `--solver`). All 7 `proofs/*.telos` exit 0 under `--solver z3`. The linear ones also
  pass under Fourier–Motzkin. The nonlinear ones fail under Fourier–Motzkin with "non-linear multiplication",
  as documented.
- **Two negative controls** in `proofs/negative/` (committed): `no_overflow_guard.telos` and
  `false_claim_sender_positive.telos`. Both exit 1 with `[FAIL]` lines. `scripts/verify_proofs.py` requires that.
- **Correction to the nightly job:** the old check (`grep "[FAIL]"`) would have failed the honest full model,
  because a disjunction group prints `[FAIL]` for each alternative branch it does not satisfy. The verdict is
  now the exit code. Exit 1 is the signal for a refuted claim.
- **Certificate** regenerated under Z3 with this binary: byte-identical to the committed `proofs/certificate.json`.
- **Changes in this pass:** `scripts/verify_proofs.py` (new); `proofs/negative/` (new); `nightly.yml` now pins
  tpt-telos to `bfa9637`, runs the runner, and also runs on push and PR to `master` (1D.03 AC: proofs on every
  push); `proofs/README.md` and `SEMVER_CONTRACTS.md` corrected (they said Z3 was not used).
- **1A.07a** checked. Only tpt-telos is pinned, since the workspace does not depend on `tpt-math` or `tpt-formal`.

Still open, and why:

- `1A.06`, `1A.07c`, `1F.02`, `1D.01`, `1D.03`: the nightly/CI jobs are updated and the proof side is verified
  locally, but none has run on GitHub. `1D.03`'s AC also needs branch protection to block merges (owner setting).
- `1C.01`, `1C.04`, `1D.04`, `1C.03`: there is no separate exact-rational ideal layer. The telos models use
  integer products (`amount*bps`), so the ideal-vs-realization split the spec asks for is not modelled. The
  integer models also do not bound values to `u64`, so the edge-case sweep (`1C.03`) is not covered by the proofs.
- `1F.01`: the spec asks the certificate to consume `tpt-formal` audit-trail output. `tpt-formal` has no such
  output in its repo, so this needs a decision on the format before it can be built.
- `1E.04`, benchmark job: needs the dedicated reference machine (`1E.01`). Stays disabled.
- `1G.03`, `1H.*`, `2C.02`, `4.05`: need named human reviewers or owner decisions.
- `2D.01`, `2D.02`, `1H.02`: owner decisions (`publish = false`, the removed `SECURITY.md`, release tag approval).
- `2A.*`, `2B.03`, `2B.04`, `2C.03`, `2D.03`, `2D.04`, `4.02`, `4.04`: not started in this pass. They are design or
  argument work (policy model, snapshot argument, N-step induction) that needs an owner decision or the
  CosmWasm source re-read. Not attempted, to avoid inventing policy.
- `3.01`–`3.06`: need an external firm, a contract and funding.

Working tree: the uncommitted Cargo.toml metadata edits and the crate README/CHANGELOG files were already there
before this pass and are not touched here. Nothing has been committed.


---

## Status notes (ninth pass: decisions applied, ideal layers)

Decisions taken on the recommendations from the previous pass:

- **1F.01 (certificate source): decided.** The certificate consumes `telos verify --json`. `tpt-formal` has no
  audit-trail output, so the spec's `tpt-formal` clause is replaced by this decision. Recorded in
  `docs/CERTIFICATE_SCHEMA.md`. A future `tpt-formal` format would be schema `0.2-draft`.
- **Ideal layer (1C, 1D.04): built, not deferred.** Separate exact models in scaled integers, so the rational
  fee needs no division by a variable (telos does not allow it):
  - `proofs/ideal_transfer_validator.telos`, PASS under Z3. Negative control `proofs/negative/ideal_creates_value.telos` FAILs.
  - `proofs/ideal_vesting.telos`, PASS under Z3. Negative control `proofs/negative/ideal_vesting_creates_value.telos` FAILs.
  - Netting is exact linear arithmetic, so its gap is identically zero. Stated in `proofs/README.md`.
- **Full run, `scripts/verify_proofs.py --solver z3`:** 9 proofs pass; 4 negative controls fail with `[FAIL]`.

Now checked: `1C.01`, `1C.04`, `1D.04`, `1F.01`, `1A.07a`.

Still open, and why:

- `1C.03`: the models are over unbounded integers, so the u64 min/max sweep is not covered by the proofs.
- `1D.01`, `1D.03`, `1F.02`, `1A.06`, `1A.07c`: proofs pass locally; the CI jobs have not run on GitHub.
  `1D.03`'s AC also needs branch protection on `master` (owner setting).
- `1G.03`, `1H.*`, `2C.02`, `4.05`, `2D.01`, `2D.02`, `1H.02`, `3.*`: need named reviewers, owner decisions or an external firm.
- `1E.01`, `1E.04`: need the reference benchmark machine.
- `2A.*`, `2B.03`, `2B.04`, `2C.03`, `2D.03`, `2D.04`, `4.02`, `4.04`: not started. These are design or argument
  work that needs an owner decision (policy model) or the CosmWasm source re-read.

---

## Status notes (tenth pass: policy implementation, split proof, audit items)

- **2A.01** done: `docs/POLICY_MODEL.md`. D1 tiered rates, D2 split by basis points, D5 Floor only. D3 rate
  limits and D4 proportional distribution are deferred; `docs/BACKLOG.md` items 14 and 15 name the blockers.
- **2A.03** done: `FeeTiers` (`crates/tpt-statera-policy/src/tiers.rs`) and `FeeShares`
  (`crates/tpt-statera-policy/src/split.rs`). The schedule and the shares are validated when built. 12 unit tests,
  including 50 000 random amounts and 100 000 random fees. Clippy with `-D warnings` is clean.
- **2A.04** partly done. `proofs/policy_split.telos` PASSes under Z3: exact conservation, non-negativity, remainder in
  `[0, 3)`, no recipient above the fee. Negative control `proofs/negative/policy_split_drops_unit.telos` FAILs. The
  **tier lookup is not proven in telos**; it is covered by tests and by the argument in POLICY_MODEL.md. So the item
  stays open until that is either proven or accepted as test-only.
- **2A.05** done. `bench/fm_netting` gained `FM_AUGMENT=policy`: the netting set plus a fixed 3333/3333/3334 bps split.
  The constraints are linear with constant shares, so Fourier–Motzkin can run them. Median 1.25 ms, p99 2.38 ms over
  5 000 calls, gate PASS, on the same provisional host as before (`bench/results/fm_policy_summary.txt`). Baseline
  netting re-run in the same session: median 0.52 ms. The augmented set is about 2.4x slower at the median.
- **1C.03** done (earlier this pass): u64 argument in `proofs/README.md`.
- **2B.03, 2B.04** done: the CosmWasm sources were re-read on 2026-11-13 (`docs/BYPASS_ARGUMENT.md`). A2 and A4 are
  retired for CosmWasm deployments in `THREAT_MODEL.md`.
- **4.02** done: `reference_settlement` generates the certificate when `TPT_TELOS` is set (verified: PASS, value conserved).
- **Transcripts** are byte-stable across runs (`scripts/verify_proofs.py` sorts telos's counterexample entries, which
  come from a hash map).

Still open, and why:

- `2A.04`: the tier-lookup proof (above).
- `1D.01`, `1D.03`, `1F.02`, `1A.06`, `1A.07c`: CI jobs not run on GitHub. `1D.03` also needs branch protection.
- `2C.02`, `2C.03`, `2C.04`, `2D.01`–`2D.04`, `3.*`, `4.04`–`4.06`, `1G.03`, `1H.*`, `1E.01`, `1E.04`: need owner
  decisions, reviewers, hardware or an external firm, as in earlier passes. `2C.03` (N-step composition) needs an
  induction argument that telos cannot express, so it is not started.
- Backlog 14 (rate limits) and 15 (proportional distribution): deferred, with named blockers.

---

## Status notes (eleventh pass: tier proof, composition induction)

- **2C.03** done. The N-step composition is an induction over the checked single-step theorem
  (proofs/README.md, "Composition over N transfers, by induction"). The induction hypothesis is the
  theorem's requires and its conclusion is its ensures. Assumptions are stated there. This supersedes the
  tenth-pass line that said 2C.03 was not started.
- **2A.04** partly done. proofs/policy_tiers_3.telos PASSes under Z3: the selection for a three-tier schedule is
  total, matches the Rust lookup, and stays in [0, 10 000]. Negative control policy_tiers_wrong_top_tier.telos
  FAILs. The generic Rust FeeTiers::rate_for cannot be proven in telos (no loops), so it stays tested, and 2A.04
  stays open for that reason.
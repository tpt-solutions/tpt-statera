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

- [ ] `1A.01` Initialize repository: `.gitignore`, `README.md` with §1 positioning + prominent status banner ("Phase 1 scope only; §2 guarantees are target architecture until runtime + multi-account composition are verified").
- [ ] `1A.02` Scaffold Cargo workspace `tpt-statera`: real members `tpt-statera-core`, `tpt-statera-proof`; placeholder stub crates `tpt-statera-policy`, `tpt-statera-runtime` (compile-empty, marked out of scope).
- [ ] `1A.03` Dual licensing: add full `LICENSE-MIT` and `LICENSE-APACHE` texts; set `license = "MIT OR Apache-2.0"` in every manifest; add `// SPDX-License-Identifier: MIT OR Apache-2.0` headers with **TPT Solutions** copyright to all source files.
- [ ] `1A.04` Toolchain decisions: Rust edition, MSRV, `rustfmt.toml`, clippy profile (deny warnings in core), `#![deny(unsafe_code)]` in core, documented arithmetic-overflow policy (deny-by-design, never wrap).
- [ ] `1A.05` Decide `no_std` posture for `tpt-statera-core` (runtime crate will target CosmWasm / Cosmos SDK); record decision + rationale in an architecture note.
- [ ] `1A.06` CI pipeline: fmt check, clippy, unit + property tests, doc build (warnings as errors), coverage; separate nightly job for proof runs + benchmarks.
- [ ] `1A.07` Dependency governance (TPT ecosystem):
  - [ ] `1A.07a` Pin exact versions of `tpt-math`, `tpt-formal`, `tpt-telos`; commit `Cargo.lock`.
  - [ ] `1A.07b` Write `SEMVER_CONTRACTS.md`: which upstream APIs/types we compile against and our bump policy.
  - [ ] `1A.07c` Upstream regression smoke tests in CI that fail loudly on behavioral drift (solver answers, math kernels).
  - [ ] `1A.07d` Decide registry vs. vendored/path fallback strategy for TPT-internal crates.

### 1B. `tpt-statera-core` Implementation (spec Appendix A)

- [ ] `1B.01` Types per Appendix A: `TransferProposal { amount, fee_rate_bps }`, `SystemState { sender_balance, recipient_balance, treasury_balance }`, `ValidationOutcome::Allow { .., dust_retained_by_sender } | Deny` — use `dust_retained_by_sender`, **never** `dust_refunded_to_sender` (§6.1 renaming rationale).
- [ ] `1B.02` Implement `validate_transfer` steps 0–6: fail-closed `fee_rate_bps > 10_000` guard; solvency check; u128 floor fee + captured `dust` excluded from `total_deduction`; u128-computed recipient/treasury sums checked against `u64::MAX` **before** any u64 addition/cast.
- [ ] `1B.03` Unit-test edge table: zero amount; zero fee; 9999 / 10000 bps; 10001 bps → Deny; sender exactly solvent vs. short by 1 unit; recipient at `u64::MAX`; treasury at `u64::MAX`; dust = 0 and dust > 0 partitions.
- [ ] `1B.04` Property tests (arbitrary valid inputs): conservation `post_sum == pre_sum` on Allow; directionality (sender decreases only, recipient/treasury increase only); Deny paths never panic or wrap (checked in release mode too).
- [ ] `1B.05` Differential oracle tests vs. `symbolic_evaluate_ideal` reference model: `|realized_fee − ideal_fee| ≤ 1.0` and ideal-Deny ⟹ realized-Deny.
- [ ] `1B.06` API docs carrying the scope notes verbatim: single-proposal/single-snapshot non-atomicity warning (§8.2); comments marking `fee_rate_bps ≤ 10000` and `sender_balance ≥ amount` as deliberate trust-boundary `requires` clauses so reviewers don't convert them to `ensures` (§6.1 final row).
- [ ] `1B.07` Demo function #1: **linear vesting calculator** — `claimable ≤ (elapsed/total) × total_allocation`, bounded by fixed-point precision, truncation dust explicitly allocated (feeds EC4).
- [ ] `1B.08` Demo function #2: **multi-party settlement netting** over N parties — Σ of all party state changes == 0; ship a reusable synthetic 10-party fixture shared with the benchmark harness (feeds EC2 + EC4).

### 1C. Ideal-Layer Verification (QF_LRA exact rationals, via `tpt-telos`)

- [ ] `1C.01` Author ideal-layer contracts for `validate_transfer`: `requires fee_rate_bps ≤ 10000`, `requires sender_balance ≥ amount` (annotated as documented trust boundaries, §8.3); `ensures` no-overdraft triple; `ensures` strict conservation `post_sum == pre_sum`.
  - **AC (part of EC1):** telos discharges both `ensures` clauses universally (FM elimination completes with nothing left unchecked); no added assumptions beyond the §8 list.
- [ ] `1C.02` Enforce the independent-clause-scope rule: no contract clause references another clause's `let` bindings; overflow clause recomputes `max_possible_fee` inline from inputs only (§6.1 rows 3 & 7).
- [ ] `1C.03` Edge-case proof sweep: zero-value transfers, u64 min/max bounds, fee extremes (0, 9999, 10000 bps) — confirm covered by the universal proof, not just by tests; archive solver transcripts as evidence artifacts.
- [ ] `1C.04` Ideal-layer contracts for the vesting calculator (temporal/proportional invariant, §2 tokenomics row).
- [ ] `1C.05` Ideal-layer contracts for netting: Σ party deltas == 0; explicitly state the constraint-orthogonality assumption and justify its validity for the 10-party fixture.
  - **AC (part of EC4):** orthogonality assumption appears in the proof artifact with a written justification.

### 1D. Realization-Layer Soundness (u64 integers, bounded rounding)

- [ ] `1D.01` Bounded-rounding `ensures`: `abs(rounded_fee − ideal_fee) ≤ 1.0`, with Floor semantics (realized fee ≤ ideal, error ∈ [0, 1)).
- [ ] `1D.02` Conservative-denial `ensures`: ideal Deny ⟹ realized Deny (**No False Allows / overdraft prevention**).
- [ ] `1D.03` Additive-overflow `ensures` in input-pure form: `(recipient_balance + amount > u64::MAX ∨ treasury_balance + max_possible_fee > u64::MAX) ⟹ Deny`, where `max_possible_fee = floor(amount × fee_rate_bps / 10000)` is recomputed inline (§6.1 rows 6–7).
  - **AC (completes EC1):** ideal (1C) + realization (1D) proofs for `validate_transfer` both pass in CI on every push; failures block merge.
- [ ] `1D.04` Two-layer soundness demonstrated on **both** extra functions (vesting + netting): paired ideal + realization contracts with per-function ideal→realization gap bounds.
  - **AC (EC4):** both functions verified in both layers; write-up documents the gap bound per function.
- [ ] `1D.05` Document rounding-mode policy: Phase 1 = **Floor only** (protocol never overcharges); `Ceiling`/`HalfEven` deferred to `tpt-statera-policy` with mandatory dust-destination routing (§5.2, Principle 2).

### 1E. FM-Elimination Scalability Benchmark (EC2)

- [ ] `1E.01` Define and freeze the **reference hardware spec** (replaces "spec TBD"): CPU model, cores, RAM, OS, solver build flags, RNG seeding; commit as `bench/REFERENCE_HARDWARE.md`.
- [ ] `1E.02` Build harness: drive FM elimination over the 10-party netting constraint set; measure per-call latency; statistical protocol (run count, warm-up, median + p99 reporting).
- [ ] `1E.03` Tractability gate: median < **10 ms** AND p99 < **50 ms** per call.
  - **AC (EC2):** passing run archived with raw latency data + hardware ID; result reproducible by a third party from the committed manifest.
- [ ] `1E.04` Nightly CI benchmark regression tracking with alert threshold (e.g., > 20% median degradation auto-opens an issue).

### 1F. First-Draft Proof Certificate (`tpt-statera-proof`, EC3)

- [ ] `1F.01` Finalize Proof Certificate format: versioned, machine-readable schema mapping verification records → control objectives; consumes `tpt-formal` audit-trail output; schema doc published.
- [ ] `1F.02` Generate first-draft certificate covering **`validate_transfer` only**.
  - **AC (EC3):** certificate regenerates deterministically in CI; every claim carries a proof ID traceable to a stored solver transcript (1C/1D artifacts).
- [ ] `1F.03` Embed the §7 caveat verbatim: illustrative artifact; not a compliance claim; excludes concurrent-access and multi-party netting scenarios; must not be presented to customers/auditors as a completed regulatory mapping until Phase 2 + Phase 3.
- [ ] `1F.04` Stub control-mapping entries for SOC2 CC6.1 and MiCA Article 51, explicitly marked "mapping pending Phase 2".

### 1G. Threat Model & Responsible-Claims Documentation (EC5)

- [ ] `1G.01` Promote spec §8 into standalone `THREAT_MODEL.md` (successor doc): 4 assumptions, serialized-validation caller obligation, trusted-config boundary, no-enforcement-bypass-yet statement.
- [ ] `1G.02` Claims-guardrail template: approved external wording ensuring "conservation of value" is never cited without its scope (single-call, serialized execution, Phase 1 boundary).
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

- [ ] `2A.01` Specify policy model: fee structures (tiered, split-recipient), rate limits, proportional distribution rules.
- [ ] `2A.02` Rounding-mode enum (`Floor`, `Ceiling`, `HalfEven`) where **every** mode declares an explicit dust destination (sender refund / treasury / burn) — no silent vanishing (Principle 2).
- [ ] `2A.03` Implement policies + property tests mirroring the 1B.03–1B.05 methodology.
- [ ] `2A.04` Telos proofs: policy definitions internally consistent; provably cannot produce negative balances or overflow (crate-map target column, §4).
- [ ] `2A.05` Re-run FM benchmark against policy-augmented constraint sets; confirm EC2 latency gates still hold; update `bench/` reports.

### 2B. `tpt-statera-runtime` Crate (FFI Bridge)

- [ ] `2B.01` Select host integration targets (CosmWasm, Cosmos SDK module, generic Rust⇄Go FFI) and design the interception ABI.
- [ ] `2B.02` Interception layer: every state transition routes through validated core/policy logic before commit.
- [ ] `2B.03` Snapshot integrity: verify host-environment state matches the verified `SystemState` snapshot at call time (anti-TOCTOU argument).
- [ ] `2B.04` Bypass-resistance: formal argument that the FFI boundary cannot be skipped (crate-map target column, §4).
- [ ] `2B.05` Close the §8.2 concurrency gap: enforce/verify serialized validation-and-commit per account (mutex / actor / blockchain nonce) at the runtime layer; update `THREAT_MODEL.md` to retire Assumption 2's caller obligation.

### 2C. Compliance Mapping (SOC2 / MiCA / Financial Audit)

- [ ] `2C.01` Control-selection matrix: SOC2 CC6.1 (Logical Access Security), MiCA Article 51 (Asset safeguarding), plus alternates legal counsel identifies ← certificate fields.
- [ ] `2C.02` Legal/compliance review of all external claims language; remove "illustrative" labels only where the delivered phases earn it.
- [ ] `2C.03` Multi-account composition verification: extend single-transition proofs to composed/portfolio-level conservation (required for §2 guarantees to apply).
- [ ] `2C.04` Produce auditor-ready evidence pack v1 in the format auditors actually consume.
  - **AC (§7 Phase 2 row):** abstract proofs converted into mapped, traceable compliance evidence reviewed by a compliance stakeholder.

### 2D. Release Engineering & Hardening

- [ ] `2D.01` Publish crates (core, proof, policy, runtime) with dual-license metadata; pre-publish `cargo package` dry-runs; evaluate + integrate dependency-vulnerability scanning.
- [ ] `2D.02` `SECURITY.md` disclosure policy with TPT Solutions security contact and response SLAs.
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

- [ ] `4.01` Define package contents: template audit submission assembled around a runnable customer-style deployment built on the Phase 1 transfer validator (§7 Phase 4 row).
- [ ] `4.02` Build the reference application end-to-end: configure system state → submit proposals → generate certificates → export the audit bundle.
- [ ] `4.03` Walkthrough guide: how customers adapt it (custom fee structures via `tpt-statera-policy`, runtime integration points, evidence regeneration).
- [ ] `4.04` Compliance packaging checklist informed by Phase 3 audit experience (what auditors requested, how it was satisfied).
- [ ] `4.05` Legal review of package claims; align caveats with post-audit reality (Phase 2 mappings delivered, Phase 3 audit complete).
- [ ] `4.06` Publish package v1; establish feedback channel; assign maintenance ownership.
  - **AC (§7 Phase 4 row):** package converts "auditable" from an abstract claim into a concrete, reusable artifact a customer can execute unmodified.
- [ ] `4.07` Backlog refresh for the next planning cycle: revisit deferred scope (non-linear vesting, additional rounding modes, multi-currency ledgers, further §2 verticals).

---

*End of checklist. Keep IDs stable when adding items — append new ones rather than renumbering.*





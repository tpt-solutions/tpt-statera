# THREAT_MODEL.md — tpt-statera

**Document version:** `0.2-draft` (todo 1G.01). Changes from 0.1: T5 partially mitigated in-process
by `tpt-statera-runtime`; the T7 restated denial clause is decided (ARCHITECTURE AD-5); T9–T10 added.
**Promoted from:** `spec.txt` §8 (Draft v1, 2026-08-20)
**Status:** DRAFT. Not signed off. See §6 (sign-off block). Phase 1 exit criterion EC5 is **not** met until §6 is completed.

This document states the assumptions under which the Phase 1 guarantees hold, and the claims that must
not be made without their scope. Any external statement of "conservation of value" must carry the scope
in §5.

## 1. Assumptions

1. **Finite, non-overflowing inputs (per field).** `validate_transfer` assumes each `TransferProposal` and
   `SystemState` field is a valid `u64`. Schema validity is an upstream obligation. A *combination* that
   overflows (for example `recipient_balance + amount`) is **not** an assumption. The function is total over
   it and returns `Deny`. This is an `ensures`-style guarantee, not a caller precondition.

2. **Serialized validation per account.** `validate_transfer` proves correctness for one call against one
   snapshot of `SystemState`. It does not provide atomicity. Two concurrent validations against the same
   sender balance can both return `Allow` and together overdraw the account. **Deploying teams must serialize
   validation-and-commit per account** (mutex, actor, or blockchain nonce). This is a caller obligation until
   `tpt-statera-runtime` closes the gap (todo 2B.05).

3. **Trusted policy inputs.** `fee_rate_bps` and `SystemState` come from a trusted configuration or state
   layer, not from an end user or an untrusted oracle. The preconditions `fee_rate_bps <= 10_000` and
   `sender_balance >= amount` are caller obligations. The implementation fails closed on both as
   belt-and-suspenders, but the primary defense is upstream trust.

4. **No enforcement-bypass guarantee yet.** Nothing in Phase 1 prevents a compromised or misconfigured host
   from skipping `validate_transfer` or ignoring a `Deny`. That is the purpose of `tpt-statera-runtime`, which
   is out of scope for Phase 1 (todo 2B.04).

## 2. Threats considered

| ID | Threat | Mitigation in Phase 1 | Residual risk |
| :--- | :--- | :--- | :--- |
| T1 | Silent u64 wraparound when adding recipient or treasury balances | Wide `u128` sums checked against `u64::MAX` before narrowing; `Deny` on overflow (`transfer_validator.rs` step 5) | None known; covered by edge-table and property tests |
| T2 | Vanishing dust from integer division | Dust computed as `numerator mod 10_000`, excluded from the sender debit, and reported as `dust_retained_by_sender` | None known; conservation property tests |
| T3 | Protocol overcharge through rounding | Floor-only rounding; realized fee ≤ ideal fee < realized fee + 1 unit | None known; `realized_fee_error_is_strictly_less_than_one_unit` |
| T4 | Misconfigured fee rate > 100% | Fail-closed guard at `fee_rate_bps > 10_000` | Trust-boundary precondition remains a caller obligation (A3) |
| T5 | Double-spend or overdraft under concurrency | **In-process:** `Ledger::submit` serializes validate-and-commit per account. **Cross-process:** a single-writer service (`tpt-statera-runtime::service`) serializes all writes, but its TCP protocol has no authentication, so it must be reached only on loopback or a private network. **On CosmWasm:** the chain serializes execution (sources in `docs/BYPASS_ARGUMENT.md`, excerpt-checked only) | **Low** for CosmWasm deployments (pending re-read of the sources); **medium** for off-chain deployments that expose the service |
| T6 | Host bypasses validation | Balances are private in `Ledger`. The CosmWasm adapter (`tpt-statera-cosmwasm`) has one state-changing message, `Transfer`, and returns an error on `Deny`. Off-chain hosts that keep a `Ledger` directly can still skip `submit` | **Medium** on CosmWasm (depends on the chain and on the instantiate caller being trusted, A3); **high** for off-chain hosts that do not route through the service |
| T7 | Divergence between the ideal model and the implementation | Differential tests; the spec's ideal-Deny clause is restated (AD-5) and tested for both gap causes | Low. The restated clause is Rust-tested; the solver does not prove it (AD-6) |
| T8 | Proof overclaim: linear model presented as a full proof | `proofs/` header comments and the certificate caveat state the scope; disjunctive clauses are excluded from the certificate | Medium: depends on how the scope is communicated externally (§5) |
| T9 | Unsoundness in the proof pipeline on disjunctive obligations or repeated assignments | Both defects were shown to be in IR lowering, shared by Fourier–Motzkin and Z3. Clauses affected are removed from the proofs and tested in Rust. Repros in `proofs/upstream-repros/` | Medium: other telos proofs that use `||`, `==>` or repeated assignment must not be trusted until upstream fixes the lowering |
| T10 | Rounding policy drift (someone adds `Ceiling` silently) | `RoundingPolicy::new` rejects `Ceiling` and `HalfEven` (`tpt-statera-policy`) | Low |

## 3. Trust boundaries

- **Input boundary.** `SystemState` and `fee_rate_bps` cross from the trusted state and config layer
  into the validator. Fields are bounded by type (`u64`). The `fee_rate_bps` bound is enforced at runtime.
- **Host boundary.** The validator returns a value. It does not enforce that the host acts on it (A4).
- **Proof boundary.** The Fourier–Motzkin solver is trusted to be sound for the QF_LRA encodings in
  `proofs/`. Solver transcripts are stored for audit.

## 4. Known limitations

- The Floor fee formula `numer = amount·bps` is a parameter of the proof, not derived. The upper half of
  bounded rounding, the overflow denial, and the restated denial clause are checked in Rust only (they
  are disjunctive, and the solver is unsound on disjunctions in this version; ARCHITECTURE AD-6).
- Vesting is not proven by the solver (it divides by a variable).
- Multi-party netting is conservation-proven only for a 10-party linear model. The
  constraint-orthogonality assumption is documented in `proofs/README.md` but not discharged.
- Benchmarks (EC2) are not run. See `bench/REFERENCE_HARDWARE.md`.

## 5. Claims guardrail (todo 1G.02)

Use these phrasings. Do **not** use the unqualified phrase "conservation of value" or "formally verified
safe" without the scope.

| Permitted | Not permitted |
| :--- | :--- |
| "Single-transfer validation with Floor rounding conserves value within one call, against one state snapshot, in the Phase 1 linear core." | "tpt-statera guarantees conservation of value." |
| "Dust is explicitly accounted for and never silently dropped." | "No value can ever be lost or created." |
| "The validator fails closed on overflow and fee-rate misconfiguration." | "The system is protected against overflow." |
| "Phase 1 proofs are illustrative and cover the linear core only." | "The system is SOC2 or MiCA compliant / mapped." |
| "Concurrent overdraft protection requires serialized validation-and-commit per account." | "Double-spends are prevented." |
| "Within one process, `Ledger::submit` serializes validate-and-commit per account." | "The ledger is protected against a host that bypasses it." |

Every external document must carry §4 (Known limitations) or a link to this file. The proof certificate
carries the caveat from `spec.txt` §7 verbatim.

## 6. Review and sign-off (todo 1G.03)

> **Not completed.** Names, roles and dates must be entered by the reviewers. This section is intentionally
> blank. Do not fill it in on a reviewer's behalf.

| Role | Name | Date | Decision (Approve / Approve with changes / Reject) |
| :--- | :--- | :--- | :--- |
| Internal reviewer 1 | *(pending)* | *(pending)* | *(pending)* |
| Internal reviewer 2 | *(pending)* | *(pending)* | *(pending)* |
| Spec owner (TPT Solutions) | *(pending)* | *(pending)* | *(pending)* |

Linked from: `README.md`. Referenced by: `proofs/certificate.json` (once the certificate schema includes the
threat-model version field; tracked under todo 1F.01).

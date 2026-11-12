# RFP Scope Draft — Independent Audit (todo 3.01)

**Status: draft for the owner.** No firm has been chosen and no contact has been made. The spec
names Trail of Bits and Halborn as examples, not as a decision.

## Scope of work

Independent review of the **Conservation of Value** and **Explicit Dust** rounding arguments (spec
§7, Phase 3), covering:

1. The conservation argument for single-transfer validation (`tpt-statera-core::validate_transfer`),
   including the restated denial clause and the Floor-rounding argument (ARCHITECTURE AD-4, AD-5).
2. The formal models in `proofs/` and the soundness of the encodings used, including the two
   solver defects recorded in ARCHITECTURE AD-6.
3. The in-process serialization in `tpt-statera-runtime`.
4. The certificate generator and its determinism.

## Materials the firm would receive (todo 3.02)

- Repository at a frozen commit (AUDIT_READINESS §1).
- Solver transcripts and the certificate.
- Benchmark harness and raw data.
- `THREAT_MODEL.md` and the known-limitations list.

## Out of scope

- Phase 2 host integration (no adapter exists yet).
- Legal and compliance mapping (counsel's scope, todo 2C.02).
- Cross-process concurrency (not implemented).

## Requested deliverables

- A written report, published with its scope caveats intact (todo 3.06).
- Findings classified critical, high, medium and low, with remediation windows.
- An attestation letter after re-review.

## Selection criteria (proposed)

- Formal-methods capability with a financial-security practice.
- Experience with rounding and conservation arguments.
- Willingness to publish with the scope caveats intact.
- Remediation windows the owner can meet.

*[Selection decision, budget and timeline: owner to set.]*

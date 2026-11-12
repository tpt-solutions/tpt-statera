# Evidence Pack (todo 1H.01 for Phase 1; 2C.04 structure for Phase 2)

Maps each Phase 1 exit criterion (spec §9, EC1–EC5) to the artifacts that evidence it, with an honest
status. **Phase 1 is not exited.** Several criteria are partly met, and EC1 and EC3 are not met as
written.

## Status summary

| EC | Requirement (spec §9) | Status | Met? |
| :--- | :--- | :--- | :---: |
| EC1 | `validate_transfer` passes ideal and realization proofs, including strict conservation and no-false-allows | Under Z3, `transfer_validator_full.telos` proves conservation, no overdraft, the no-op denial, both rounding bounds, overflow denial and the restated denial. The ideal layer is the Floor relation, not a separate exact-rational model | **Partly** (ideal layer not modelled separately) |
| EC2 | FM benchmark on 10-party netting: median < 10 ms, p99 < 50 ms on reference hardware | Gate **passes** on the frozen reference host, for the netting constraint shape. Not a third-party reproduction. Caveats in `bench/REFERENCE_HARDWARE.md` | **Partly** (hardware not dedicated; shape only) |
| EC3 | First-draft Proof Certificate, mapped to SOC2/MiCA, labelled illustrative | Certificate generated under Z3 from the full model: 16 PASS claims (including the disjunctive ones), deterministic, caveat verbatim. Mappings are stubs | **Partly** (mapping is a stub) |
| EC4 | Two-layer soundness on ≥2 functions beyond `validate_transfer`; netting proof addresses orthogonality | Netting: proven, orthogonality **argued** in writing (`proofs/README.md`). Vesting: proven under Z3 for one claim (`vesting_exact.telos`) and for two claims over time (`vesting_schedule.telos`) | **Partly** (more than two claims by induction, not checked) |
| EC5 | Threat-model document reviewed and signed off | Document drafted (`THREAT_MODEL.md` 0.2-draft). **Sign-off not recorded** | **No** |

## Artifacts

| Artifact | Path | Evidences |
| :--- | :--- | :--- |
| Core implementation and tests (48 tests across the workspace) | `crates/tpt-statera-core`, `crates/tpt-statera-policy`, `crates/tpt-statera-runtime` | EC1 (Rust side), EC4 |
| Full model (every clause, exact product) | `proofs/transfer_validator_full.telos` | EC1, 1D.01–1D.03 |
| Conservation model (linear clauses, history) | `proofs/transfer_validator.telos` | EC1 (superseded) |
| Netting model | `proofs/netting_10party.telos` | EC4 |
| Composition model | `proofs/composition_two_transfers.telos` | 2C.03 (partial) |
| Negative controls | described in `proofs/README.md` | Non-vacuity of EC1 and EC4 |
| Solver transcripts | `proofs/transcripts/transfer_validator_full.json` | EC3 traceability |
| Certificate | `proofs/certificate.json` | EC3 |
| Benchmark raw data and summary | `bench/results/fm_netting_raw.csv`, `bench/results/fm_netting_summary.txt` | EC2 |
| Hardware record | `bench/REFERENCE_HARDWARE.md` | EC2 |
| Threat model | `THREAT_MODEL.md` | EC5 (pending sign-off) |

## Open items blocking exit

1. **EC1:** the realization-layer and overflow proofs need a solver that handles disjunctions, or a
   restructured encoding. The upstream defect is recorded in `docs/ARCHITECTURE.md` AD-6.
2. **EC2:** a dedicated reference machine (`bench/REFERENCE_HARDWARE.md`), and a benchmark over
   coupled solvency shapes, not just the box-shaped one.
3. **EC3:** certificate mappings beyond stubs need counsel review (Phase 2).
4. **EC4:** vesting needs a solver that handles division by a variable.
5. **EC5:** named reviewers and dates in `THREAT_MODEL.md` §6. This item cannot be completed by the
   code and must not be filled in on a reviewer's behalf.
6. **1H.02:** tagging the release is not done. `CHANGELOG.md` is drafted, and a tag and any push need
   the owner's approval.
7. **1H.03:** the spec revision history row for AD-5 is added. The Appendix A comment is still to be
   updated by the spec owner.

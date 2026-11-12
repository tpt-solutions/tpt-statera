# Control-Selection Matrix (todo 2C.01)

**Status: draft for compliance review. No mapping in this file is approved.** Every entry that
claims a link to a control is marked by its evidence status. Legal and compliance review (todo 2C.02)
owns the final wording.

Columns: the control; what `tpt-statera` can contribute; the artifact that would evidence it; and
the current status.

| Control | Source | What tpt-statera can contribute | Evidence artifact | Status |
| :--- | :--- | :--- | :--- | :--- |
| Logical access security | SOC2 CC6.1 | Balances are private. The only write path is `Ledger::submit` (in-process access control by visibility, not by identity) | `crates/tpt-statera-runtime/src/lib.rs`, tests | Mapping pending Phase 2. Identity and authorization are **not** in this codebase, so CC6.1 is largely the host's responsibility |
| Asset safeguarding | MiCA Art. 51 | Conservation of value within one call and one state snapshot (linear core). Dust accounted for | `proofs/certificate.json`, `proofs/transcripts/transfer_validator.json` | Mapping pending Phase 2. Scope is the Phase 1 linear core only, per the spec §7 caveat |
| Change management / integrity of computation | SOC2 CC8.1 (alternate, to confirm) | Deterministic certificate generation; committed transcripts; negative controls | `scripts/generate_certificate.py`, `proofs/` | Candidate. Not yet mapped. Needs counsel to confirm the criterion |
| Monitoring / logging | SOC2 CC7.2 (alternate, to confirm) | Submission outcomes are returned as typed values (`Committed`, `Denied`, `Stale`); no logging yet | none yet | Gap. No audit log is written by the runtime |
| Accuracy of financial records | Financial audit standards (to confirm) | Conservation tests; fixed-point arithmetic with no silent wrap | test suites in `crates/*/tests` | Candidate. Not yet mapped |

## Gaps the matrix exposes

1. **No identity or authorization** in the codebase. Any control that depends on "who submitted this"
   belongs to the host.
2. **No audit log.** The runtime returns results but records nothing durable.
3. **Phase 1 linear-core scope only.** Anything beyond single-transfer conservation (for example
   concurrent overdraft across processes, or disjunctive clauses) is outside the proofs.
4. **Certificate stubs.** SOC2 CC6.1 and MiCA Art. 51 mappings in the certificate are stubs marked
   "mapping pending Phase 2".

## Process

Counsel and the compliance stakeholder (todo 2C.02) review this matrix. An entry moves from
"Candidate" to "Mapped" only with a signed review recorded in `docs/EVIDENCE_PACK.md`.

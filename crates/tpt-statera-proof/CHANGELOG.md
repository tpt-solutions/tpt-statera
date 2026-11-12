# Changelog

All notable changes to `tpt-statera-proof`. The workspace-level history is in
[../../CHANGELOG.md](../../CHANGELOG.md).

## Unreleased (0.1.0 candidate)

### Added

- Solver-free verifier for `proofs/certificate.json` (todo 5D.01): schema, status, caveat, verdicts,
  input SHA-256, proof-ID binding to the source hash, and transcript presence.
- `docs/certificate.schema.json` (todo 5D.02), with a test that keeps its fields in step with the certificate.
- Scope caveat constant (`ILLUSTRATIVE_CAVEAT`), unchanged.

### Known limitations

- The schema has no transcript hashes, so transcript contents are not verified. Adding them is schema `0.2-draft`.
- The verifier does not run the solver. It checks consistency, not the proofs.

### Not released

- No tag has been created. Tagging needs the owner's approval.

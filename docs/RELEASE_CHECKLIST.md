# Release Checklist (todo 4.04, informed by the Phase 3 audit once it exists)

Run this before any tagged release. Each step has a command or an artifact to check. A step that
cannot be done is marked, and the release is not cut until it is resolved or formally accepted.

## Code

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace` and `cargo test --workspace --release`
- [ ] `cargo doc --workspace --no-deps` with no warnings
- [ ] Every source file has an SPDX header (`LICENSE-MIT` and `LICENSE-APACHE` present)
- [ ] `Cargo.lock` committed and current

## Proofs

- [ ] `telos verify` passes for every file in `proofs/` that is not marked superseded
- [ ] Each model has its negative control, and the control FAILs (`proofs/README.md`)
- [ ] `scripts/generate_certificate.py` run twice gives identical output
- [ ] `proofs/certificate.json` `overall` is `PASS`, and its caveat matches spec §7 verbatim

## Benchmarks

- [ ] `bench/fm_netting` run with `FM_RAW_CSV` and the summary updated
- [ ] Hardware record matches the machine that produced the numbers (`bench/REFERENCE_HARDWARE.md`)

## Claims

- [ ] Every external statement carries the scope from `THREAT_MODEL.md` §5
- [ ] No claim of a compliance mapping; the certificate is labelled illustrative
- [ ] `THREAT_MODEL.md` version is the one the release cites

## Governance

- [ ] Threat-model sign-off recorded (EC5). **Requires named reviewers. Do not fill in on their behalf.**
- [ ] Legal review of claims (todo 2C.02, 4.05) recorded
- [ ] `CHANGELOG.md` updated, claims inside the caveat
- [ ] Tag and any push approved by the owner

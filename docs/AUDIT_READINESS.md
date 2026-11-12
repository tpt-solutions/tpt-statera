# Audit-Readiness Package (todo 3.03)

**Status: draft.** The package is ready to assemble. It is not frozen, and the repo is not yet
tagged, so no snapshot exists. Freezing needs the owner's approval (`docs/RELEASE_CHECKLIST.md`).

## 1. Frozen snapshot

- Commit hash of the frozen state, plus the `tpt-telos` commit it was verified against
  (currently `23d53eb`).
- `Cargo.lock` committed.
- Tag name: *[to be chosen by the owner]*.

## 2. Reproduction instructions

```sh
cargo test --workspace
cargo run -p tpt-statera-runtime --example reference_settlement -- out
telos verify proofs/transfer_validator.telos
telos verify proofs/composition_two_transfers.telos
telos verify proofs/netting_10party.telos
python scripts/generate_certificate.py --telos <path-to-telos>
cargo run --release --manifest-path bench/fm_netting/Cargo.toml   # requires the sibling tpt-telos checkout
```

Expected results: all tests pass; each `telos verify` reports `all constraints satisfied`; the
certificate reports `overall: PASS` with 10 claims; the benchmark prints its gate line.

## 3. What the auditor should look at first

1. The restated denial clause (`docs/ARCHITECTURE.md` AD-5) and its differential test.
2. The solver defects and workarounds (AD-6): disjunctions, repeated assignments.
3. The overflow and rounding guards in `crates/tpt-statera-core/src/transfer_validator.rs`.
4. The lock-and-commit path in `crates/tpt-statera-runtime/src/lib.rs`.

## 4. Known-limitations list (lifted from `THREAT_MODEL.md`)

- Concurrency is serialized in-process only.
- A host can bypass the validator.
- Disjunctive clauses are not machine-checked.
- Vesting is not machine-checked.
- Benchmark hardware is not dedicated.
- Certificate mappings are stubs.

## 5. Evidence to request from the auditor

Scope questions to put to the auditor, to avoid an open-ended engagement: confirm the conservation
argument for the linear core; confirm the Floor-rounding argument; review the in-process lock order for
deadlock freedom; review the claims guardrail (`THREAT_MODEL.md` §5).

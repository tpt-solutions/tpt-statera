# Changelog

All notable changes. Claims in this file stay inside the spec §7 caveat and `THREAT_MODEL.md` §5.

## Unreleased (0.1.0 candidate)

### Added

- `tpt-statera-core`: `validate_transfer` (Floor fee, explicit dust, deny-on-overflow and
  deny-on-insolvency), linear vesting calculator, atomic 10-party netting, shared fixtures.
  `#![no_std]`, zero dependencies.
- `tpt-statera-policy`: rounding policy with mandatory dust destinations (Floor only), tiered fee
  rates (`FeeTiers`), and fee splits by basis points (`FeeShares`) with a treasury remainder, so the
  parts sum to the fee exactly. Not yet called by `validate_transfer` or the runtime.
- `tpt-statera-runtime`: in-process `Ledger` with serialized validate-and-commit per account, and a
  single-writer service with a TCP line protocol for cross-process use. The service requires a shared
  token and has no TLS, so use it on loopback or a private network only.
- `tpt-statera-runtime` examples: payments, tokenomics vesting, supply-chain netting, DeFi pool payouts,
  reference settlement with an audit bundle, and `ledger_server` / `ledger_client`. The reference
  settlement run regenerates the certificate when `TPT_TELOS` is set.
- `tpt-statera-cosmwasm`: CosmWasm contract (`cosmwasm-std` 3.0.11). `Transfer` is the only state-changing
  message. Builds for `wasm32` (371 KB). Unit-tested and tested with `cw-multi-test`; not yet run on a chain.
- `proofs/`: `transfer_validator.telos` (conservation, no overdraft, never-overcharge),
  `netting_10party.telos` (conservation), `composition_two_transfers.telos` (two-step net effect).
- `proofs/`: `policy_tiers_3.telos` (three-tier fee selection) and `policy_split.telos` (fee split
  conservation). Both PASS under Z3, each with a FAIL negative control (todo 2A.04, partly done).
- `proofs/README.md`: N-step composition argued by induction over the checked single-step theorem
  (todo 2C.03). The argument is written, not machine-checked, and it states three assumptions.
- `scripts/verify_proofs.py` runs every proof and requires every negative control to fail. The nightly
  workflow runs it on push and pull request to `master`, and on schedule. It has not yet run on GitHub.
- First-draft Proof Certificate (`proofs/certificate.json`, 16 claims, illustrative), generated under Z3 from `transfer_validator_full.telos`.
- Frozen candidate reference hardware and the FM netting benchmark. The gate passes on that host for the
  netting shape, and again on the policy-augmented set (13 obligations per call, median 1.25 ms, p99 2.38 ms; todo 2A.05).
- `THREAT_MODEL.md` 0.2-draft, control matrix, evidence pack,
  walkthrough, release checklist, backlog, audit-readiness draft, RFP scope draft.
- Packaging metadata, and a per-crate `README.md` and `CHANGELOG.md`.
- `.gitattributes`, which normalises line endings.

### Changed

- The spec's ideal-Deny clause is restated (ARCHITECTURE AD-5). The original was false under Floor
  rounding. Checked by `tests/differential.rs`.
- The superseded abstract-fee model (`proofs/transfer_conservation.telos`) is removed. Its replacement is
  `transfer_validator_full.telos`.
- Nightly proof job: tpt-telos is pinned to commit `bfa9637`, not the default branch, and the verdict is
  the exit code of the verifier.

### Known limitations

- Cross-process service uses a shared token but no TLS. Use it on loopback or a private network only.
- The CosmWasm adapter has not run on a live chain.
- The Fourier–Motzkin solver does not prove disjunctive clauses (upper rounding bound, overflow denial,
  restated denial). Under `--solver z3` the full model proves them. Vesting is proven under Z3 for one
  claim and two claims over time; more than two claims is by induction, not checked.
- N-step composition is an induction argued in `proofs/README.md`, not checked by telos. It assumes
  serialized steps, fee rates within `[0, 10 000]`, and that a `Deny` step is a no-op.
- Policy tiers and splits are proven for a three-tier schedule and a two-recipient split only. Their
  generic Rust code is tested, not proven.
- Serialization is in-process only. Host-level bypass is not mitigated.
- The threat-model sign-off is not recorded.
- Certificate mappings are stubs. The certificate is illustrative.
- `Ceiling` and `HalfEven` rounding are not permitted.

### Not released

- No tag has been created, and nothing has been pushed. Both need the owner's approval.

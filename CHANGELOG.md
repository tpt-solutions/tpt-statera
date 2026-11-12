# Changelog

All notable changes. Claims in this file stay inside the spec §7 caveat and `THREAT_MODEL.md` §5.

## Unreleased (0.1.0 candidate)

### Added

- `tpt-statera-core`: `validate_transfer` (Floor fee, explicit dust, deny-on-overflow and
  deny-on-insolvency), linear vesting calculator, atomic 10-party netting, shared fixtures.
  `#![no_std]`, zero dependencies.
- `tpt-statera-policy`: rounding policy with mandatory dust destinations. Floor only.
- `tpt-statera-runtime`: in-process `Ledger` with serialized validate-and-commit per account, and a
  single-writer service with a TCP line protocol for cross-process use (no authentication: loopback only).
- `tpt-statera-cosmwasm`: CosmWasm contract (`cosmwasm-std` 3.0.11). `Transfer` is the only state-changing
  message. Builds for `wasm32` (371 KB). Unit-tested; not yet run on a chain.
- Worked examples: payments, tokenomics vesting, supply-chain netting, reference settlement with
  an audit bundle.
- `proofs/`: `transfer_validator.telos` (conservation, no overdraft, never-overcharge),
  `netting_10party.telos` (conservation), `composition_two_transfers.telos` (two-step net effect).
- First-draft Proof Certificate (`proofs/certificate.json`, 16 claims, illustrative), generated under Z3 from `transfer_validator_full.telos`.
- Frozen candidate reference hardware and the FM netting benchmark (gate passes on that host).
- `THREAT_MODEL.md` 0.2-draft, control matrix, evidence pack,
  walkthrough, release checklist, backlog, audit-readiness draft, RFP scope draft.

### Changed

- The spec's ideal-Deny clause is restated (ARCHITECTURE AD-5). The original was false under Floor
  rounding. Checked by `tests/differential.rs`.

### Known limitations

- Cross-process service uses a shared token but no TLS. Use it on loopback or a private network only.
- The CosmWasm adapter has not run on a live chain.

- The solver does not prove disjunctive clauses (upper rounding bound, overflow denial, restated
  denial), so these are Rust-tested only. Vesting is not proven.
- Serialization is in-process only. Host-level bypass is not mitigated.
- The threat-model sign-off is not recorded.
- Certificate mappings are stubs. The certificate is illustrative.
- `Ceiling` and `HalfEven` rounding are not permitted.

### Not released

- No tag has been created, and nothing has been pushed. Both need the owner's approval.

# Changelog

All notable changes to `tpt-statera-runtime`. The workspace-level history is in
[../../CHANGELOG.md](../../CHANGELOG.md).

## Unreleased (0.1.0 candidate)

### Added

- `Ledger::with_policy`: tier rates and fee shares paid to named fee accounts from the treasury, in the same locked commit (todo 5B.08, POLICY_MODEL D6).
- In-process `Ledger` with serialized validate-and-commit per account, versioned snapshots and a
  `Stale` check before commit.
- Single-writer `WriterService` with a TCP line protocol (`serve_tcp`, `TcpClient`). Requires a shared token.
- Examples: `payments`, `tokenomics_vesting`, `supply_chain_netting`, `defi_pool_payouts`,
  `reference_settlement`, plus `ledger_server` and `ledger_client`.
- The `reference_settlement` run regenerates the proof certificate when `TPT_TELOS` is set, and records
  the certificate's source and verdict in its audit bundle.

### Known limitations

- In-process guarantee only. A host that holds a `Ledger` can bypass `submit` (spec §8, Assumption 4).
- The TCP service uses a shared token but no TLS. Use it on loopback or a private network only.
- Process-local serialization; no cross-process atomicity.
- Multi-transfer composition is not enforced here. The N-step argument is in `proofs/README.md` (todo 2C.03).
- This crate depends on `tpt-statera-policy` but does not use it yet.

### Not released

- No tag has been created. Tagging needs the owner's approval.

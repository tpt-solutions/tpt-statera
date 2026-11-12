# Changelog

All notable changes to `tpt-statera-cosmwasm`. The workspace-level history is in
[../../CHANGELOG.md](../../CHANGELOG.md).

## Unreleased (0.1.0 candidate)

### Added

- CosmWasm contract on `cosmwasm-std` 3.0.11. `Transfer` is the only state-changing message.
- Builds for `wasm32` (371 KB). Unit-tested with `cw-multi-test`.
- `library` feature to disable the exported entry points.

### Known limitations

- Not yet run on a live chain.
- Instantiate-only configuration: no admin message, no funding after instantiation.

### Not released

- No tag has been created. Tagging needs the owner's approval.

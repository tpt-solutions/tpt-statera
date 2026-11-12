# Backlog for the Next Planning Cycle (todo 4.07)
Items deferred from Phase 1–2, in the order the evidence says they matter. Each names the blocker,
so the next cycle can start from the reason, not the title.
| # | Item | Blocker | Spec / todo ref |
| :-- | :--- | :--- | :--- |
| 1 | Realization proofs (upper rounding bound, overflow denial, restated denial) | Done under `--solver z3` in `proofs/transfer_validator_full.telos` (tpt-telos `60d44cf` fixed the disjunction grouping). Remaining: a separate exact-rational ideal layer, if needed | 1D.01–1D.03 |
| 2 | Vesting, as an exact model | Done under `--solver z3`: the claim step (`vesting_exact.telos`) and two claims over time (`vesting_schedule.telos`). More than two claims is by induction, not checked | 1C.04, 1D.04 |
| 3 | Nonlinear proofs | Done under `--solver z3` with monomials: `floor_fee_exact.telos`, `vesting_exact.telos`. Default solver still refuses them | AD-6 fourth-pass update |
| 4 | Dedicated benchmark machine and coupled-solvency benchmark | Hardware not provided | 1E.01, 1E.02 |
| 5 | CosmWasm adapter | Built and unit-tested, with simulator tests (`cw-multi-test`). `cosmwasm-check` does not build on Windows (wasmer singlepass). A live-node run is still open | 2B.01, 2B.02 |
| 6 | Host-level bypass argument | Written for CosmWasm, with three cited claims checked against excerpts only (`docs/BYPASS_ARGUMENT.md`) | 2B.04 |
| 7 | Cross-process serialization | Single-writer service with a shared-token line protocol, verified across processes. No TLS: loopback or a private network with the token | 2B.05 |
| 8 | Ceiling and HalfEven rounding with a sub-unit refund accumulator | Design of the accumulator | 2A.02, AD-4 |
| 9 | Audit log in the runtime | Not designed | 2C.01 gap |
| 10 | N-step composition by induction in the solver | Composition is proven for two steps | 2C.03 |
| 11 | Certificate mappings beyond stubs | Counsel review | 2C.02, 1F.04 |
| 12 | Multi-currency ledgers and non-linear vesting | Deferred scope | spec §2, 4.07 |
| 13 | Generated code returns a function's trailing value | Done for i64 returns in the Rust and Go backends (tpt-telos `bfa9637`), verified by rustc and go build. Other return types are unchanged | telos codegen |
| 14 | Policy rate limits (per-account, windowed) | Needs a clock source and a per-account counter store in `tpt-statera-runtime`. The validator is stateless, so it cannot hold them (`docs/POLICY_MODEL.md` D3) | 2A.01 |
| 15 | Policy proportional distribution | Overlaps the netting and vesting proofs; a generic policy rule needs its own proofs first (`docs/POLICY_MODEL.md` D4) | 2A.01 |

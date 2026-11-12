# Backlog for the Next Planning Cycle (todo 4.07)
Items deferred from Phase 1–2, in the order the evidence says they matter. Each names the blocker,
so the next cycle can start from the reason, not the title.
| # | Item | Blocker | Spec / todo ref |
| :-- | :--- | :--- | :--- |
| 1 | Realization proofs (upper rounding bound, overflow denial, restated denial) | Done under `--solver z3` in `proofs/transfer_validator_full.telos` (tpt-telos `60d44cf` fixed the disjunction grouping). Remaining: a separate exact-rational ideal layer, if needed | 1D.01–1D.03 |
| 2 | Vesting, as an exact model | Done under `--solver z3`: `proofs/vesting_exact.telos` (the claim step). Remaining: a full schedule over time, if needed | 1C.04, 1D.04 |
| 3 | Nonlinear proofs | Done under `--solver z3` with monomials: `floor_fee_exact.telos`, `vesting_exact.telos`. Default solver still refuses them | AD-6 fourth-pass update |
| 4 | Dedicated benchmark machine and coupled-solvency benchmark | Hardware not provided | 1E.01, 1E.02 |
| 5 | CosmWasm adapter | Built and unit-tested (`tpt-statera-cosmwasm`). Still to do: live-chain or `cw-multi-test` run, `cosmwasm-check`, and re-read of the cited CosmWasm pages | 2B.01, 2B.02 |
| 6 | Host-level bypass argument | Written for CosmWasm, with three cited claims checked against excerpts only (`docs/BYPASS_ARGUMENT.md`) | 2B.04 |
| 7 | Cross-process serialization | Single-writer service with a TCP line protocol is built and verified across processes. Missing: authentication, TLS, authorization | 2B.05 |
| 8 | Ceiling and HalfEven rounding with a sub-unit refund accumulator | Design of the accumulator | 2A.02, AD-4 |
| 9 | Audit log in the runtime | Not designed | 2C.01 gap |
| 10 | N-step composition by induction in the solver | Composition is proven for two steps | 2C.03 |
| 11 | Certificate mappings beyond stubs | Counsel review | 2C.02, 1F.04 |
| 12 | Multi-currency ledgers and non-linear vesting | Deferred scope | spec §2, 4.07 |
| 13 | Generated code does not return a function's trailing value | The code generators emit it as an expression statement, and the generated signature has no return type. Needs a codegen change for return types and tail returns (verification is unaffected) | telos codegen |

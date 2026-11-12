# proofs/ — tpt-telos models for tpt-statera

Machine-checked models of the Phase 1 arithmetic, written in the `tpt-telos` language
(`requires` / `ensures` / `mutate state`), checked by the built-in Fourier–Motzkin solver.

**Read `docs/ARCHITECTURE.md` AD-6 first.** It explains the encoding choices and a solver defect
found by negative control. Disjunctive `ensures` clauses are **not** trusted from this telos version.

## Index

| File | Status | Proves (PASS) | Does not prove |
| :--- | :--- | :--- | :--- |
| `transfer_validator.telos` | **PASS**, certificate source | Strict conservation; no overdraft; directionality; never-overcharge (`credited·10000 ≤ numer`) — all on both Allow and Deny | Deny is a no-op; upper half of bounded rounding; overflow denial; restated denial clause (AD-5). These are disjunctive and are checked in Rust (`tests/differential.rs`). The Floor formula `numer = amount·bps` is a parameter, not derived. |
| `transfer_conservation.telos` | **Superseded** by `transfer_validator.telos`. Kept as history | Conservation with an abstract fee and a parametric cap | Fee formula, guards, status |
| `composition_two_transfers.telos` | **PASS** (2C.03, partial) | Conservation and non-negativity across two sequential transfer steps, as the net effect, with step 2's solvency on step 1's result | Two steps only. The net-effect form is a deliberate workaround (AD-6): repeated assignments in separate `mutate` blocks are not reliable in this solver version. N-step composition is by induction, not checked. |
| `netting_10party.telos` | **PASS** | Conservation under a zero-sum 10-party settlement; non-negativity of parties 0 and 9 (invariants for all ten) | Constraint orthogonality is argued, not proven (below) |

### Negative controls (each must FAIL)

| Control | Mutation | Result |
| :--- | :--- | :--- |
| `transfer_validator.telos` with treasury credited `fee + 1` | one-unit error in the credit | FAIL on conservation and on never-overcharge |
| `transfer_validator.telos` with the overflow guard removed | Allow without the treasury cap check | **PASS (wrong)**. This is the solver defect in AD-6, so the disjunctive overflow clause is not proven. |

The first control shows the conjunctive proof is not vacuous. The second is why the overflow clause is not claimed.

### Reproduce

```sh
telos verify proofs/transfer_validator.telos
telos verify proofs/netting_10party.telos
python scripts/generate_certificate.py --telos path/to/telos   # writes certificate.json + transcripts/
```

## Certificate

`certificate.json` is the first-draft Proof Certificate (todo 1F.02), generated from
`transfer_validator.telos`. It has 10 claims (6 ensures, 4 invariants), one per obligation. The
claims are deduplicated across control-flow paths, and a claim passes only if every path passes.
Each claim carries a `proof_id` that traces to `transcripts/transfer_validator.json`. The generator
is deterministic: two runs give byte-identical output. Control mappings for SOC2 CC6.1 and MiCA
Article 51 are stubs marked "mapping pending Phase 2" (todo 1F.04). The caveat is the spec §7 text,
verbatim.

The certificate scope is `validate_transfer` under the Floor-fee specification. It does **not**
include the disjunctive clauses, so the certificate does not claim the overflow denial or the
restated denial clause.

## Not yet proven (open items)

- **Upper half of bounded rounding** (todo 1D.01): `credited·10000 > numer − 10000` on Allow. This
  is disjunctive, so it is checked in Rust only.
- **Overflow denial** (todo 1D.03): disjunctive, Rust-tested only.
- **Restated denial** (todo 1D.02, AD-5): disjunctive, Rust-tested only.
- **Floor formula derivation**: `numer = amount·bps` is a parameter. Discharging it needs Z3 or a
  bilinear-capable solver.
- **Vesting** (todo 1C.04, 1D.04): `floor(elapsed·alloc / total)` divides by a variable. Blocked
  without a bilinear-capable solver.

## Constraint-orthogonality assumption (todo 1C.05)

This is a written argument. The solver does not produce it. A reviewer must check it under todo 1G.03.

**Assumption.** The netting conservation claim depends only on the zero-sum equality
`d0 + … + d9 == 0`. The per-party solvency constraints `b_i + d_i >= 0` are *orthogonal* to it: they
couple no pair of parties beyond the shared equality, and they do not change the derivation of the total.

**Justification for the 10-party fixture.**

1. The conservation ensures is derived from the zero-sum `requires` alone. Summing the ten `mutate`
   assignments gives `Σ b_i' = Σ b_i + Σ d_i`, and `Σ d_i = 0` is the only term that needs a constraint.
   The solvency `requires` are not used in that derivation, so they cannot affect the conservation result.
2. The solvency `requires` are used only for the non-negativity ensures (`b0 >= 0`, `b9 >= 0`). Each
   involves a single party's balance and delta, so the constraints are box-shaped per variable. This is the
   case in which the FM elimination order cannot create cross-party coupling.
3. The fixture in `tpt-statera-core::fixtures` satisfies every solvency constraint with the pre-settlement
   balances, and the Rust test `ten_party_settlement_conserves_total_balance` checks conservation on it.

**Limit.** This covers the linear 10-party model with box-shaped solvency constraints. It does not cover
settlements whose solvency bounds couple several parties, such as joint limits or a shared pool. Such a
case must re-open this item.

## Versions

- `tpt-telos` 0.2.0 (sibling checkout `tpt-telos`, release build of `telos.exe`, commit `23d53eb`).
- Solver: built-in Fourier–Motzkin (no Z3).

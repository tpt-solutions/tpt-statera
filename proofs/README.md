# proofs/ — tpt-telos models for tpt-statera

Machine-checked models of the Phase 1 arithmetic, written in the `tpt-telos` language
(`requires` / `ensures` / `mutate state`), checked by the built-in Fourier–Motzkin solver.

**Read `docs/ARCHITECTURE.md` AD-6 first.** It explains the encoding choices and the defects found by
negative control. Those defects are in IR lowering, shared by both solvers (Fourier–Motzkin and Z3). Disjunctive `ensures` clauses are **not** trusted from this telos version.

## Index

| File | Status | Proves (PASS) | Does not prove |
| :--- | :--- | :--- | :--- |
| `transfer_validator_full.telos` | **PASS** under `--solver z3`, certificate source | Every clause of `validate_transfer` with the exact Floor product: conservation, no overdraft, directionality, no-op denial, both rounding bounds, overflow denial, and the restated denial (AD-5). Needs the fixed disjunction semantics (commit `60d44cf` in tpt-telos). | Nothing in the spec is left unproven here. The ideal layer is the Floor relation, not a separate exact-rational model. |
| `transfer_validator.telos` | **PASS** (linear clauses only) | Strict conservation; no overdraft; directionality; never-overcharge (`credited·10000 ≤ numer`) — all on both Allow and Deny | Deny is a no-op; upper half of bounded rounding; overflow denial; restated denial clause (AD-5). These are disjunctive and are checked in Rust (`tests/differential.rs`). The Floor formula `numer = amount·bps` is a parameter, not derived. |
| `composition_two_transfers.telos` | **PASS** (2C.03, partial) | Conservation and non-negativity across two sequential transfer steps, as the net effect, with step 2's solvency on step 1's result | Two steps only. The net-effect form is a deliberate workaround (AD-6): repeated assignments in separate `mutate` blocks are not reliable in this solver version. N-step composition is by induction, not checked. |
| `netting_10party.telos` | **PASS** | Conservation under a zero-sum 10-party settlement; non-negativity of parties 0 and 9 (invariants for all ten) | Constraint orthogonality is argued, not proven (below) |
| `ideal_transfer_validator.telos` | **PASS** under `--solver z3` (ideal layer, 1C.01) | Exact conservation, no overdraft, directionality, Deny is a no-op, with the fee kept exact in scaled integers (balances × 10000, fee = `amount·bps`) | Ideal layer only. Its link to the realization layer is the gap bound in `transfer_validator_full.telos` (below) |
| `ideal_vesting.telos` | **PASS** under `--solver z3` (ideal layer, 1C.04) | Exact conservation, no overdraft, claim ≤ allocation, claim exactly `elapsed·alloc` (balances scaled by `total`) | Ideal layer only. Gap bound against `vesting_exact.telos` (below) |

### Ideal and realization layers (todo 1C, 1D.04)

Each function has an ideal model (exact, scaled integers, no division) and a realization model (floored):

| Function | Ideal (exact) | Realization (floored) | Gap, ideal − realized (raw units) |
| :--- | :--- | :--- | :--- |
| `validate_transfer` fee | `ideal_transfer_validator.telos`, fee = `amount·bps` in 1/10000 units | `transfer_validator_full.telos`, `10000·fee ≤ amount·bps < 10000·fee + 10000` | `[0, 1)` |
| Linear vesting | `ideal_vesting.telos`, claim = `elapsed·alloc` in 1/total units | `vesting_exact.telos`, `v·total ≤ elapsed·alloc < v·total + total` | `[0, 1)` |
| 10-party netting | `netting_10party.telos` (no rounding; the model is exact) | the same model | `0` identically |

The ideal Deny implies the realized Deny, or a shortfall under one unit (`transfer_validator_full.telos` E7,
AD-5). The ideal layers are not linked to the realization layers in one file, so each link is stated here,
and each bound is proven in its own file.

### Negative controls (each must FAIL)

| Control | Mutation | Result |
| :--- | :--- | :--- |
| `transfer_validator.telos` with treasury credited `fee + 1` | one-unit error in the credit | FAIL on conservation and on never-overcharge |
| `transfer_validator_full.telos` with the overflow guard removed | Allow without the treasury cap check | FAIL under Z3. Before the fix in tpt-telos `60d44cf` this passed. The defect was in the verifier's disjunction grouping, not in the solvers. Earlier text:  AD-6, so the disjunctive overflow clause is not proven. |

The first control shows the conjunctive proof is not vacuous. The second is why the overflow clause is not claimed.

### Reproduce

```sh
# Every proof must pass, and every proofs/negative/ control must refute a claim (exit non-zero).
# The telos binary must be built with `--features z3` (tpt-telos, crate tpt-telos-cli, package `tpt-telos`).
python scripts/verify_proofs.py --telos path/to/telos --solver z3
python scripts/generate_certificate.py --telos path/to/telos --solver z3   # writes certificate.json + transcripts/
```

Read the exit code, not the text. A disjunction group prints `[FAIL]` for each alternative branch
it does not satisfy, and still exits 0 when the group holds on one branch.

## Certificate

`certificate.json` is the first-draft Proof Certificate (todo 1F.02), generated from
`transfer_validator_full.telos`, under Z3 (`python scripts/generate_certificate.py --solver z3`). It has 16 claims: 10 obligations (invariants and conjunctive ensures) and 6 disjunctions, one per obligation or group. The
claims are deduplicated across control-flow paths, and a claim passes only if every path passes.
Each claim carries a `proof_id` that traces to `transcripts/transfer_validator_full.json`. The generator
is deterministic: two runs give byte-identical output. Control mappings for SOC2 CC6.1 and MiCA
Article 51 are stubs marked "mapping pending Phase 2" (todo 1F.04). The caveat is the spec §7 text,
verbatim.

The certificate scope is `validate_transfer` under the Floor-fee specification. It does **not**
include the disjunctive clauses, so the certificate does not claim the overflow denial or the
restated denial clause.

## Not yet proven (open items)

- **Floor fee as a derivation.** The Floor relation is a specification, not a derivation from the rounding
  operator. The relation itself, with the exact product, is in `transfer_validator_full.telos`.
- **Ideal layer as a separate model.** Done for the transfer and vesting (see the table above). The ideal and
  realized layers are still two files, so the link between them is a stated bound, not a single proof.
- **Vesting over time.** Only the claim step is modelled (`vesting_exact.telos`).
- **Repeated assignment.** Refused by telos. Models here assign each variable once per path, as the verifier requires.

## Integer domain and the u64 edge cases (todo 1C.03)

The models quantify over all integers, not over `u64`. Telos cannot write the literal `2^64 − 1`
(integer literals are limited to i64), so the bounds are symbolic (`cap`, `total`, `big`).

Why this covers the `u64` edge cases:

- Every `u64` value is an integer, so a universal proof over `Int` also holds for every `u64` input.
  The `requires` only add `>= 0` and the caps, which every `u64` satisfies.
- The edge cases named in the todo are instances of the universal proof, not separate facts:
  zero amount (`amount >= 0` includes 0), fee rate 0 and 10000 (`bps` range), a sender exactly solvent or
  one unit short (the Allow/Deny split in `transfer_validator_full.telos`), and recipient or treasury at
  the cap (E6, the overflow denial, with `cap` symbolic).
- The one thing the integer model does not show is that the Rust code does not wrap at `u64::MAX`. That is
  the job of the checked `u128` arithmetic in `tpt-statera-core` and its edge-table tests (1B.03). The
  differential test (1B.05) ties the Rust result to the ideal reference.

Solver transcripts for every model and control are in `proofs/transcripts/verify/`, written by
`scripts/verify_proofs.py`.

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

## Nonlinear proofs (monomials, `--solver z3`)

Products of two variables are now supported. They are exact under `telos verify --solver z3`, and the
default Fourier-Motzkin solver refuses them, so a default run cannot report a nonlinear proof as passed.

| File | Status | Proves (Z3, all PASS) | Does not prove |
| :--- | :--- | :--- | :--- |
| `floor_fee_exact.telos` | **PASS** under `--solver z3` | Conservation; no overdraft; directionality; never-overcharge with the exact product `amount*bps` | Upper half of bounded rounding and overflow denial (disjunctive, lowering defect AD-6). The Floor relation is a requires, i.e. the specification, not a derivation from the rounding operator. |
| `vesting_schedule.telos` | **PASS** under `--solver z3` | Two claims at times e1 <= e2: the vested amount is monotone, the payments sum to the vested amount at e2, the total is within the allocation, and each vested amount is within its ideal proportion. A negative control (no e2 >= e1) FAILs. |  The Floor relation is a specification. Schedules of more than two claims follow by the same argument, not checked here. |
| `vesting_exact.telos` | **PASS** under `--solver z3` | Conservation of claimed plus pool; claimed never exceeds the ideal proportion; truncation below one unit; claimed at most the allocation | The vesting schedule as a whole (the claim is one step). Floor is the defining relation, as above. |

Negative controls, each FAIL under Z3 as expected: `floor_fee_exact` with the credit off by one (fails
conservation and never-overcharge); `vesting_exact` with the claim off by one (fails four clauses).

Upstream repro of the earlier rejection, now fixed for `--solver z3`: `upstream-repros/nonlinear_product_rejected.telos`.
Its default-solver run still reports `non-linear multiplication is not supported`, by design.

The telos change set is saved as `upstream-repros/tpt-telos-monomials.patch`. It covers the IR monomial term,
the exact-mode switch, the Z3 encoding and the Fourier-Motzkin refusal, plus the z3 0.12 API port.

## Z3 backend

The linear proofs above verify the same under `--solver z3` and the default solver.

Upstream repros (`proofs/upstream-repros/`):

- `nonlinear_product_rejected.telos`: rejected by the default solver; verifies under `--solver z3`.
- `repeated_assignment_neg_control_passes.telos`: a broken model that used to pass. It is now refused with `field ... is assigned more than once on one path` (tpt-telos `980a92f`). The earlier cause was a bare expression statement that was lowered as an assignment of `0`, which made the premises contradictory.
- `disjunction_overflow_neg_control_passes.telos`: a broken model that used to pass under both solvers. It now FAILs under both, since the verifier's disjunction fix (`60d44cf`).
- `tpt-telos-z3-api-0.12.patch`: the port of `z3_solver.rs` to the z3 0.12 API needed to build the feature.
- `tpt-telos-monomials.patch`: the monomial change set (see above).

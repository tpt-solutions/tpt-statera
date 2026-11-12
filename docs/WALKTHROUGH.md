# Walkthrough: adapting tpt-statera (todo 4.03)

This guide is for an integrator who wants to build a deployment on the Phase 1 validator. It uses
only the APIs that exist in the repo. Read `THREAT_MODEL.md` first. Its assumptions are the
integrator's obligations.

## 1. Pick what you need

| Need | Crate | Entry point |
| :--- | :--- | :--- |
| One transfer, validated against a snapshot | `tpt-statera-core` | `validate_transfer` |
| Fee with explicit dust routing (Floor only) | `tpt-statera-policy` | `RoundingPolicy::new(Floor, dest)?.split_fee` |
| Atomic validate-and-commit in one process | `tpt-statera-runtime` | `Ledger::submit` |
| Linear vesting schedule | `tpt-statera-core` | `vesting::claimable` |
| Atomic multi-party settlement | `tpt-statera-core` | `netting::apply_settlement` |

Worked, runnable examples live in `crates/tpt-statera-runtime/examples/`:

```sh
cargo run -p tpt-statera-runtime --example payments
cargo run -p tpt-statera-runtime --example tokenomics_vesting
cargo run -p tpt-statera-runtime --example supply_chain_netting
cargo run -p tpt-statera-runtime --example reference_settlement -- <out-dir>
```

## 2. Configure trusted inputs

`fee_rate_bps` and the starting balances come from your config or state layer (THREAT_MODEL A3).
`Ledger::new` takes them as arguments. It does not validate `fee_rate_bps <= 10_000`, so check it
before construction. The validator denies values above that bound, but the fee rate is trusted
config and should be checked where it is set.

## 3. Submit and handle every outcome

```rust
use tpt_statera_runtime::{Ledger, Submission};

let ledger = Ledger::new(&[(1, 1_000_000), (2, 0)], 15);
match ledger.submit(1, 2, 500)? {
    Submission::Committed(outcome) => { /* record outcome */ }
    Submission::Denied(reason) => { /* no state changed; reason is a DenyReason, e.g. InsufficientBalance */ }
    Submission::Stale => { /* a version moved; resubmit if you want */ }
}
```

Handle all three. `Denied` is not an error. It is the validator's answer.

## 4. Keep the serialization guarantee

`Ledger::submit` serializes validate-and-commit **within one process**. If you run more than one
process against the same balances, you need your own serialization (a database transaction, a chain
nonce, or one actor). That is a deployment obligation (THREAT_MODEL T5).

## 5. Check conservation in your own pipeline

`Ledger::total()` returns the sum of all balances under every lock. Assert that it is unchanged across
each batch. The reference application does this and writes the result to its manifest.

## 6. Regenerate the evidence

```sh
telos verify --solver z3 proofs/transfer_validator_full.telos
python scripts/generate_certificate.py --telos path/to/telos
```

Keep the certificate and transcripts with the release. The certificate's caveat must stay with it
(spec §7). Do not present the certificate as a compliance mapping (see `docs/CONTROL_MATRIX.md`).

## 7. What you cannot use yet

- **`Ceiling` and `HalfEven` rounding.** `RoundingPolicy::new` rejects them (ARCHITECTURE AD-4).
- **Cross-process concurrency control.** Not provided (see step 4).
- **Host-level enforcement.** Nothing forces your host to call `submit` (THREAT_MODEL A4). The
  CosmWasm adapter is not built.

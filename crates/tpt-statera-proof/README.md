# tpt-statera-proof

A solver-free verifier for the first-draft Proof Certificate (`proofs/certificate.json`).

> **Status: illustrative.** The certificate is an illustrative artifact (spec §7). It is not a compliance
> claim. This crate checks that the certificate is *consistent with its inputs*. It does not re-run the
> solver, and it does not show that the proofs hold.

## What it checks

| Check | Meaning |
| :--- | :--- |
| `schema` | The schema version is the one this crate understands. Any other version is refused, not guessed at. |
| `status`, `caveat` | `ILLUSTRATIVE`, and the spec §7 caveat verbatim. |
| `claims`, `overall` | At least one claim, and every claim and the overall verdict are `PASS`. |
| `proof_ids_unique` | No two claims share a proof ID. |
| `input <path>` | Each input file exists under the repository root and hashes to its recorded SHA-256. |
| `proof_id_binding` | Each proof ID carries the first 8 hex digits of its source's SHA-256 (see `docs/CERTIFICATE_SCHEMA.md`). |
| `transcripts_present` | Each claim's transcript file exists. The schema has no transcript hashes, so their contents are not checked. |

Unknown fields are a parse error. A new field is a new schema version.

## Use

```rust
use std::path::Path;
let report = tpt_statera_proof::verify_certificate_file(
    Path::new("proofs/certificate.json"),
    Path::new("."),
).unwrap();
assert!(report.passed(), "{report}");
```

Or from the command line: `cargo run -p tpt-statera-cli -- cert verify proofs/certificate.json --root .`

The schema is in `docs/certificate.schema.json`. A test keeps its field names in step with the committed certificate.

## Where the certificate comes from

`scripts/generate_certificate.py` produces `proofs/certificate.json` from `tpt-telos` runs. See
[../../proofs/README.md](../../proofs/README.md) for the models and solver requirements.

## Testing

```sh
cargo test -p tpt-statera-proof
```

Licensed under MIT OR Apache-2.0. See [CHANGELOG.md](CHANGELOG.md) for release history.

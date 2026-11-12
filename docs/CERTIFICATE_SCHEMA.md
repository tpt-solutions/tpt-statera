# Proof Certificate Schema — `tpt-statera.proof-certificate/0.1-draft` (todo 1F.01)

Produced by `scripts/generate_certificate.py`. The file is `proofs/certificate.json`, and each
run's raw solver output is in `proofs/transcripts/<model>.json`.

## Top-level fields

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `schema` | string | Always `tpt-statera.proof-certificate/0.1-draft` for this version. A change to any field is a new schema version. |
| `status` | string | Always `ILLUSTRATIVE` in Phase 1 (spec §7). |
| `caveat` | string | The spec §7 caveat, verbatim. Must not be removed or paraphrased. |
| `scope` | string | What the claims cover, and what they exclude. |
| `toolchain` | object | The solver used. `solver` names the engine. |
| `inputs` | array | One entry per proof source: `{path, sha256}`. The hash binds the certificate to the exact `.telos` text. |
| `claims` | array | One entry per obligation (see below). |
| `overall` | string | `PASS` only if every claim passes. |
| `control_mappings` | object | Map from a control name to its status. Phase 1 values are the stub `mapping pending Phase 2`. |

## Claim fields

| Field | Meaning |
| :--- | :--- |
| `proof_id` | `TSC-<first 8 hex of the source sha256>-<NNN>`. Stable as long as the source does not change. |
| `source` | Path of the `.telos` file. |
| `function` | Function name the obligation belongs to. |
| `line` | Source line of the `ensures` or invariant. |
| `description` | The obligation, as the solver printed it. |
| `result` | `PASS` or `FAIL`. |
| `path_instances` | Number of control-flow paths the solver checked this obligation on. Collapsed into one claim. |
| `transcript` | The transcript file that holds the raw output. |

## Determinism

The generator sorts keys, uses fixed ordering, and strips absolute paths from transcripts. Two runs on
the same inputs give byte-identical output (checked: `sha256` of certificate and transcript matched).

## Deduplication rule

The solver reports an obligation once per control-flow path, and names the path variants
`function[branch k]`. The generator strips the suffix, groups the entries by
`(function, line, description)`, and sets `result` to `PASS` only if every instance passed.

## Known gaps

- **tpt-formal audit trail not consumed.** The spec (todo 1F.01) asks the certificate to consume
  `tpt-formal` output. It does not yet. The certificate reads only `telos verify --json`.
- **Control mappings are stubs.** SOC2 CC6.1 and MiCA Article 51 are marked `mapping pending Phase 2`.
- **Disjunctive clauses are excluded.** A claim is not made for a clause the solver cannot check
  reliably (`docs/ARCHITECTURE.md` AD-6).
- **Threat-model version not referenced.** The certificate does not yet carry the `THREAT_MODEL.md`
  version. Adding it is a schema change.

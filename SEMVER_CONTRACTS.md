# SEMVER_CONTRACTS.md

Upstream APIs and types this project compiles against, and the policy for bumping them
(todo 1A.07b).

## Runtime dependencies

| Crate | Version | Used by | Notes |
| :--- | :--- | :--- | :--- |
| *(none)* | — | `tpt-statera-core` | Zero dependencies. Phase 1 does not use `tpt-math`. |

If `tpt-math` is adopted, add a row here with the exact pinned version, and state which of its APIs
are used.

## External verification tool (not a Cargo dependency)

| Tool | Pinned version | Used by | Pin mechanism |
| :--- | :--- | :--- | :--- |
| `tpt-telos` (`telos` CLI) | `0.2.0` (workspace version in sibling checkout) | `proofs/*.telos`, `scripts/generate_certificate.py` | Version recorded in `proofs/README.md`; the solver transcript records the version in use |
| `tpt-telos` Z3 backend (`--solver z3`, build with `--features z3`) | as above | all proofs, and the certificate | Required for the nonlinear proofs (products). Fourier–Motzkin refuses them; see `docs/ARCHITECTURE.md` AD-6 |

The CLI surface relied on is exactly:

- `telos verify --solver <z3|fourier-motzkin> <file>`: the exit code is the verdict (0 = all claims hold).
- `telos verify <file> --json`: the output schema consumed by `scripts/generate_certificate.py`
  (`passed`, `functions[].checks[].{description, passed, location.line}`).

A change to that JSON schema, or to the `requires`/`ensures`/`mutate state` semantics, is a breaking change.

## Bump policy

- **Patch bump of `tpt-telos`:** rerun all proofs and the certificate generator. The results must match
  the committed transcripts. A mismatch is a regression to investigate, not a transcript to regenerate.
- **Minor or major bump of `tpt-telos`:** also re-review each `proofs/*.telos` file for semantic change, and
  record the bump in the changelog.
- **`tpt-statera-core` public API:** pre-1.0. Breaking changes are allowed in minor versions, but each one
  must be listed in `CHANGELOG.md`.

## Regression smoke tests (todo 1A.07c)

The `ci.yml` nightly job is the place to detect upstream behavioral drift. The job runs `telos verify`
over every committed proof and fails when any `[FAIL]` line appears or the certificate changes.

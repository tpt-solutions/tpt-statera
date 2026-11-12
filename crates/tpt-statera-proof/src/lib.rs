// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Solver-free verifier for the first-draft Proof Certificate (todo 1F, 5D.01).
//!
//! The certificate is produced by `scripts/generate_certificate.py` from `tpt-telos` runs. This crate
//! does not run a solver. It checks the committed `proofs/certificate.json` against the files it names:
//!
//! * the schema version, status and caveat match the values this crate expects;
//! * the overall verdict is `PASS`, and every claim is `PASS`;
//! * every input file exists under the repository root, and its SHA-256 matches;
//! * every claim's `source` is one of the inputs;
//! * every `proof_id` carries the first 8 hex digits of its source's SHA-256 (the binding in
//!   `docs/CERTIFICATE_SCHEMA.md`), and no two claims share an ID;
//! * every claim's `transcript` file exists.
//!
//! ## What this does not check
//!
//! * It does **not** re-run the solver. A passing verdict here means the certificate is consistent with
//!   its inputs. It does not mean the proofs hold. Re-run `scripts/verify_proofs.py` for that.
//! * The schema has no hash for transcripts, so their contents are not verified, only their presence.
//!   Adding transcript hashes is a schema change (`0.2-draft`).
//! * The certificate is illustrative. Its caveat is checked verbatim and must not be removed.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Caveat text that must accompany any first-draft certificate, verbatim from spec §7 (todo 1F.03).
pub const ILLUSTRATIVE_CAVEAT: &str = "Phase 1's \"first-draft Proof Certificate\" is an illustrative artifact demonstrating the certificate format and workflow. It is not, by itself, a compliance claim, does not cover concurrent-access or multi-party netting scenarios (Section 8), and should not be represented to customers or auditors as a completed regulatory mapping until Phase 2 is delivered and Phase 3's independent audit has occurred.";

/// The schema version this verifier understands. A different value is refused, not guessed at.
pub const SCHEMA_VERSION: &str = "tpt-statera.proof-certificate/0.1-draft";

/// The only status allowed in Phase 1.
pub const STATUS: &str = "ILLUSTRATIVE";

/// A parsed certificate. Unknown fields are refused, because a new field is a new schema version.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Certificate {
    /// The caveat, verbatim.
    pub caveat: String,
    /// One entry per obligation.
    pub claims: Vec<Claim>,
    /// Control name to status.
    pub control_mappings: BTreeMap<String, String>,
    /// Proof sources and their hashes.
    pub inputs: Vec<Input>,
    /// `PASS` only if every claim passes.
    pub overall: String,
    /// Schema version.
    pub schema: String,
    /// What the claims cover and exclude.
    pub scope: String,
    /// `ILLUSTRATIVE` in Phase 1.
    pub status: String,
    /// The solver used.
    pub toolchain: Toolchain,
}

/// One proof source.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Path relative to the repository root.
    pub path: String,
    /// Hex SHA-256 of the file's bytes.
    pub sha256: String,
}

/// The solver that produced the claims.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Toolchain {
    /// For example `tpt-telos verify --solver z3`.
    pub solver: String,
}

/// One obligation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// The obligation, as the solver printed it.
    pub description: String,
    /// Function the obligation belongs to.
    pub function: String,
    /// Obligation kind.
    pub kind: String,
    /// Source line.
    pub line: u64,
    /// Number of control-flow paths checked.
    pub path_instances: u64,
    /// `TSC-<8 hex of source sha256>-<NNN>`.
    pub proof_id: String,
    /// `PASS` or `FAIL`.
    pub result: String,
    /// The `.telos` source, relative to the repository root.
    pub source: String,
    /// The transcript file, relative to the repository root.
    pub transcript: String,
}

/// One check and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// Short name of the check.
    pub name: String,
    /// Whether it passed.
    pub passed: bool,
    /// What was found, for a reader of the report.
    pub detail: String,
}

/// The full report. It passes only if every check passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Every check, in the order it ran.
    pub checks: Vec<Check>,
}

impl Report {
    /// `true` if every check passed.
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// The checks that failed.
    pub fn failures(&self) -> impl Iterator<Item = &Check> {
        self.checks.iter().filter(|c| !c.passed)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in &self.checks {
            let mark = if c.passed { "PASS" } else { "FAIL" };
            writeln!(f, "[{mark}] {}: {}", c.name, c.detail)?;
        }
        write!(
            f,
            "{}",
            if self.passed() {
                "certificate consistent with its inputs"
            } else {
                "certificate NOT consistent with its inputs"
            }
        )
    }
}

/// Errors that stop verification before any check runs.
#[derive(Debug)]
pub enum VerifyError {
    /// The certificate is not valid JSON, or does not match the schema shape.
    Parse(serde_json::Error),
    /// The certificate file could not be read.
    Io(std::io::Error),
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::Parse(e) => write!(f, "certificate does not parse: {e}"),
            VerifyError::Io(e) => write!(f, "cannot read certificate: {e}"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of the canonical text form: every CRLF becomes LF.
///
/// Git stores the `.telos` sources with LF. A Windows checkout with CRLF line endings would otherwise
/// hash differently from the stored bytes, and the certificate would not verify on one platform or the
/// other. Hashing the canonical form makes the check the same on every checkout. The generator
/// (`scripts/generate_certificate.py`) uses the same rule.
pub fn canonical_sha256(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    sha256_hex(&out)
}

/// Verify a certificate given as JSON text. `root` is the repository root that the paths are relative to.
pub fn verify_certificate(json: &str, root: &Path) -> Result<Report, VerifyError> {
    let cert: Certificate = serde_json::from_str(json).map_err(VerifyError::Parse)?;
    Ok(verify_parsed(&cert, root))
}

/// Verify the certificate at `path`. Paths inside it are resolved against `root`.
pub fn verify_certificate_file(path: &Path, root: &Path) -> Result<Report, VerifyError> {
    let json = fs::read_to_string(path).map_err(VerifyError::Io)?;
    verify_certificate(&json, root)
}

fn verify_parsed(cert: &Certificate, root: &Path) -> Report {
    let mut checks = Vec::new();
    let mut push = |name: &str, passed: bool, detail: String| {
        checks.push(Check {
            name: name.to_string(),
            passed,
            detail,
        });
    };

    push(
        "schema",
        cert.schema == SCHEMA_VERSION,
        format!("found `{}`, expected `{SCHEMA_VERSION}`", cert.schema),
    );
    push(
        "status",
        cert.status == STATUS,
        format!("found `{}`", cert.status),
    );
    push(
        "caveat",
        cert.caveat == ILLUSTRATIVE_CAVEAT,
        "caveat matches spec §7 verbatim".to_string(),
    );

    let failing: Vec<&str> = cert
        .claims
        .iter()
        .filter(|c| c.result != "PASS")
        .map(|c| c.proof_id.as_str())
        .collect();
    push(
        "claims",
        !cert.claims.is_empty() && failing.is_empty(),
        format!(
            "{} claims, {} not PASS{}",
            cert.claims.len(),
            failing.len(),
            if failing.is_empty() {
                String::new()
            } else {
                format!(": {}", failing.join(", "))
            }
        ),
    );
    push(
        "overall",
        cert.overall == "PASS" && failing.is_empty(),
        format!("overall verdict `{}`", cert.overall),
    );

    // Proof IDs are unique.
    let mut seen = BTreeMap::new();
    for c in &cert.claims {
        *seen.entry(c.proof_id.as_str()).or_insert(0u32) += 1;
    }
    let dupes: Vec<&str> = seen
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(id, _)| *id)
        .collect();
    push(
        "proof_ids_unique",
        dupes.is_empty(),
        if dupes.is_empty() {
            "no duplicate proof IDs".to_string()
        } else {
            format!("duplicates: {}", dupes.join(", "))
        },
    );

    // Inputs exist and hash to the recorded value. `actual` holds the hash of each readable file. The
    // proof-ID binding is checked against it, so a tampered recorded hash fails only its own check.
    let mut actual: BTreeMap<&str, String> = BTreeMap::new();
    for input in &cert.inputs {
        let file = root.join(&input.path);
        match fs::read(&file) {
            Ok(bytes) => {
                let hash = canonical_sha256(&bytes);
                let ok = hash.eq_ignore_ascii_case(&input.sha256);
                let detail = if ok {
                    format!("{} matches", input.path)
                } else {
                    format!(
                        "{}: recorded {}, file is {}",
                        input.path, input.sha256, hash
                    )
                };
                push(&format!("input {}", input.path), ok, detail);
                actual.insert(input.path.as_str(), hash);
            }
            Err(e) => push(
                &format!("input {}", input.path),
                false,
                format!("{}: cannot read ({e})", input.path),
            ),
        }
    }

    // Every claim's source is a readable input, and its proof ID binds to that input's actual hash.
    let mut source_problems = Vec::new();
    for c in &cert.claims {
        let prefix = proof_id_prefix(&c.proof_id);
        match actual.get(c.source.as_str()) {
            None => source_problems.push(format!(
                "{}: source `{}` is not a readable input",
                c.proof_id, c.source
            )),
            // `get` rather than slicing: a short hash must fail a check, not panic.
            Some(hash) if prefix.is_none() || hash.get(..8) != prefix.as_deref() => {
                source_problems.push(format!(
                    "{}: prefix does not match the source hash",
                    c.proof_id
                ));
            }
            Some(_) => {}
        }
    }
    push(
        "proof_id_binding",
        source_problems.is_empty(),
        if source_problems.is_empty() {
            format!(
                "all {} proof IDs bind to their source hash",
                cert.claims.len()
            )
        } else {
            source_problems.join("; ")
        },
    );

    // Transcripts exist. The schema does not hash them, so contents are not checked.
    let missing: Vec<&str> = cert
        .claims
        .iter()
        .map(|c| c.transcript.as_str())
        .filter(|t| !root.join(t).is_file())
        .collect();
    let mut unique_missing = missing.clone();
    unique_missing.sort_unstable();
    unique_missing.dedup();
    push(
        "transcripts_present",
        missing.is_empty(),
        if missing.is_empty() {
            "every transcript file exists (contents are not hashed by this schema)".to_string()
        } else {
            format!("missing: {}", unique_missing.join(", "))
        },
    );

    Report { checks }
}

/// The 8 hex digits in `TSC-<8 hex>-<NNN>`, or `None` if the ID has another shape.
fn proof_id_prefix(id: &str) -> Option<String> {
    let rest = id.strip_prefix("TSC-")?;
    let (hex, num) = rest.split_once('-')?;
    let well_formed = hex.len() == 8
        && hex.bytes().all(|b| b.is_ascii_hexdigit())
        && num.len() == 3
        && num.bytes().all(|b| b.is_ascii_digit());
    well_formed.then(|| hex.to_ascii_lowercase())
}

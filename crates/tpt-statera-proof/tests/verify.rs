// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The verifier against the committed certificate, and against tampered copies of it.

use std::path::{Path, PathBuf};

use serde_json::Value;
use tpt_statera_proof::{sha256_hex, verify_certificate, Report};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed() -> Value {
    let text = std::fs::read_to_string(repo_root().join("proofs/certificate.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Apply `edit` to the committed certificate and verify the result.
fn verify_edited(edit: impl FnOnce(&mut Value)) -> Result<Report, tpt_statera_proof::VerifyError> {
    let mut v = committed();
    edit(&mut v);
    verify_certificate(&v.to_string(), &repo_root())
}

fn failed_names(r: &Report) -> Vec<String> {
    r.failures().map(|c| c.name.clone()).collect()
}

#[test]
fn committed_certificate_verifies_against_the_repository() {
    let r = verify_certificate(&committed().to_string(), &repo_root()).unwrap();
    assert!(r.passed(), "\n{r}");
}

#[test]
fn sha256_matches_the_standard_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn altered_input_hash_fails_the_input_check() {
    let r = verify_edited(|v| v["inputs"][0]["sha256"] = Value::from("00".repeat(32))).unwrap();
    assert!(!r.passed());
    assert_eq!(failed_names(&r).len(), 1, "{r}");
    assert!(failed_names(&r)[0].starts_with("input "));
}

#[test]
fn a_claim_that_is_not_pass_fails() {
    let r = verify_edited(|v| v["claims"][0]["result"] = Value::from("FAIL")).unwrap();
    assert_eq!(
        failed_names(&r),
        vec!["claims".to_string(), "overall".to_string()],
        "{r}"
    );
}

#[test]
fn an_altered_caveat_fails() {
    let r =
        verify_edited(|v| v["caveat"] = Value::from("This is a compliance certificate.")).unwrap();
    assert_eq!(failed_names(&r), vec!["caveat".to_string()], "{r}");
}

#[test]
fn an_unbound_proof_id_fails_the_binding_check() {
    let r =
        verify_edited(|v| v["claims"][0]["proof_id"] = Value::from("TSC-00000000-001")).unwrap();
    assert!(
        failed_names(&r).contains(&"proof_id_binding".to_string()),
        "{r}"
    );
}

#[test]
fn a_malformed_proof_id_fails_the_binding_check() {
    let r = verify_edited(|v| v["claims"][0]["proof_id"] = Value::from("nope")).unwrap();
    assert!(
        failed_names(&r).contains(&"proof_id_binding".to_string()),
        "{r}"
    );
}

#[test]
fn duplicate_proof_ids_fail() {
    let r = verify_edited(|v| {
        let first = v["claims"][0]["proof_id"].clone();
        v["claims"][1]["proof_id"] = first;
    })
    .unwrap();
    assert!(
        failed_names(&r).contains(&"proof_ids_unique".to_string()),
        "{r}"
    );
}

#[test]
fn a_missing_input_file_fails_its_check() {
    let r = verify_edited(|v| v["inputs"][0]["path"] = Value::from("proofs/does_not_exist.telos"))
        .unwrap();
    assert!(
        failed_names(&r).iter().any(|n| n.starts_with("input ")),
        "{r}"
    );
    assert!(
        failed_names(&r).contains(&"proof_id_binding".to_string()),
        "{r}"
    );
}

#[test]
fn a_malformed_recorded_hash_fails_without_panicking() {
    let r = verify_edited(|v| v["inputs"][0]["sha256"] = Value::from("ab")).unwrap();
    assert!(!r.passed());
}

#[test]
fn an_unknown_field_is_refused_at_parse_time() {
    let mut v = committed();
    v["extra"] = Value::from(true);
    assert!(verify_certificate(&v.to_string(), &repo_root()).is_err());
}

#[test]
fn a_different_schema_version_fails() {
    let r = verify_edited(|v| v["schema"] = Value::from("tpt-statera.proof-certificate/0.2-draft"))
        .unwrap();
    assert_eq!(failed_names(&r), vec!["schema".to_string()], "{r}");
}

#[test]
fn canonical_hash_ignores_crlf_line_endings() {
    use tpt_statera_proof::canonical_sha256;
    assert_eq!(canonical_sha256(b"a\r\nb\r\n"), sha256_hex(b"a\nb\n"));
    assert_eq!(canonical_sha256(b"a\nb\n"), sha256_hex(b"a\nb\n"));
    // A bare CR is not a line ending and is kept.
    assert_eq!(canonical_sha256(b"a\rb"), sha256_hex(b"a\rb"));
}

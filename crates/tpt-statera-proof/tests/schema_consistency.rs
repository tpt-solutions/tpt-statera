// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `docs/certificate.schema.json` must agree with the committed certificate and with the crate's constants.
//! There is no JSON Schema validator in the build, so this checks the parts that can drift: field names,
//! required lists, and the constant values.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;
use tpt_statera_proof::{ILLUSTRATIVE_CAVEAT, SCHEMA_VERSION, STATUS};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(path)).unwrap()).unwrap()
}

fn keys(v: &Value) -> BTreeSet<String> {
    v.as_object().unwrap().keys().cloned().collect()
}

fn set(v: &Value) -> BTreeSet<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn schema_fields_match_the_committed_certificate() {
    let schema = read("docs/certificate.schema.json");
    let cert = read("proofs/certificate.json");

    assert_eq!(keys(&schema["properties"]), keys(&cert));
    assert_eq!(set(&schema["required"]), keys(&cert));

    let claim_keys = keys(&cert["claims"][0]);
    assert_eq!(keys(&schema["$defs"]["claim"]["properties"]), claim_keys);
    assert_eq!(set(&schema["$defs"]["claim"]["required"]), claim_keys);

    let input_keys = keys(&cert["inputs"][0]);
    assert_eq!(keys(&schema["$defs"]["input"]["properties"]), input_keys);
    assert_eq!(set(&schema["$defs"]["input"]["required"]), input_keys);
}

#[test]
fn schema_constants_match_the_crate() {
    let schema = read("docs/certificate.schema.json");
    assert_eq!(schema["properties"]["schema"]["const"], SCHEMA_VERSION);
    assert_eq!(schema["properties"]["status"]["const"], STATUS);
    assert_eq!(schema["properties"]["caveat"]["const"], ILLUSTRATIVE_CAVEAT);
}

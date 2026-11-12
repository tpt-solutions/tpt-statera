// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! End-to-end checks for the `statera` binary (todo 5C.03). Exit codes and printed output are the contract.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::str::contains;

fn statera() -> Command {
    Command::cargo_bin("statera").unwrap()
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn validate_allows_and_prints_the_fee() {
    statera()
        .args([
            "validate",
            "--sender",
            "1000",
            "--recipient",
            "0",
            "--amount",
            "500",
            "--fee-bps",
            "100",
        ])
        .assert()
        .code(0)
        .stdout("allow: fee 5, dust 0 retained by sender\n");
}

#[test]
fn validate_denies_with_the_reason_and_exit_1() {
    statera()
        .args([
            "validate",
            "--sender",
            "1000",
            "--recipient",
            "0",
            "--amount",
            "1000",
            "--fee-bps",
            "100",
        ])
        .assert()
        .code(1)
        .stdout(contains("insufficient_for_fee"));
}

#[test]
fn missing_arguments_are_a_usage_error_with_exit_2() {
    statera()
        .args(["validate", "--sender", "1"])
        .assert()
        .code(2);
}

#[test]
fn policy_lint_accepts_a_valid_policy() {
    statera()
        .args(["policy", "lint"])
        .arg(fixture("policy_ok.toml"))
        .assert()
        .code(0)
        .stdout(contains("valid"));
}

#[test]
fn policy_lint_reports_every_problem_in_one_run() {
    let out = statera()
        .args(["policy", "lint"])
        .arg(fixture("policy_bad.toml"))
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("rounding:"), "{text}");
    assert!(text.contains("tiers:"), "{text}");
    assert!(text.contains("split:"), "{text}");
}

#[test]
fn policy_files_refuse_unknown_keys() {
    statera()
        .args(["policy", "lint"])
        .arg(fixture("policy_unknown_key.toml"))
        .assert()
        .code(2)
        .stderr(contains("rate_limit"));
}

#[test]
fn simulate_fees_shows_tier_rate_and_split() {
    // 500 is in the first tier (100 bps): fee 5, of which account 7 takes floor(5 * 40%) = 2 and the treasury 3.
    // 2 000 is in the second tier (50 bps): fee 10, account 7 takes 4 and the treasury 6.
    statera()
        .args(["simulate-fees", "--policy"])
        .arg(fixture("policy_ok.toml"))
        .args(["500", "2000"])
        .assert()
        .code(0)
        .stdout(contains("500\t100\t5\t3\t7=2"))
        .stdout(contains("2000\t50\t10\t6\t7=4"));
}

#[test]
fn committed_certificate_verifies() {
    statera()
        .args(["cert", "verify"])
        .arg(repo_root().join("proofs/certificate.json"))
        .arg("--root")
        .arg(repo_root())
        .assert()
        .code(0)
        .stdout(contains("certificate consistent with its inputs"));
}

#[test]
fn tampered_certificate_fails_verification() {
    let text = std::fs::read_to_string(repo_root().join("proofs/certificate.json")).unwrap();
    let tampered = text.replacen("\"result\": \"PASS\"", "\"result\": \"FAIL\"", 1);
    assert_ne!(text, tampered);
    let path = std::env::temp_dir().join(format!("statera-tampered-{}.json", std::process::id()));
    std::fs::write(&path, tampered).unwrap();
    statera()
        .args(["cert", "verify"])
        .arg(&path)
        .arg("--root")
        .arg(repo_root())
        .assert()
        .code(1)
        .stdout(contains("NOT consistent"));
    let _ = std::fs::remove_file(&path);
}

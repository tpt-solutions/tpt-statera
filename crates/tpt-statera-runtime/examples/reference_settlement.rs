// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Reference application (todo 4.01–4.02): configure state → submit proposals → certify →
//! export an audit bundle.
//!
//! Run: `cargo run -p tpt-statera-runtime --example reference_settlement -- <out-dir>`
//!
//! The bundle contains `manifest.txt` (the run log), the proof certificate, and the
//! solver transcript from `proofs/`. Certificate generation is a separate step that needs
//! the `telos` binary (`scripts/generate_certificate.py`). The bundle copies whatever is
//! committed, and it records the certificate's `overall` verdict so an auditor can tell
//! whether it was generated from the current proofs.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use tpt_statera_core::ValidationOutcome;
use tpt_statera_runtime::{Ledger, Submission, TREASURY};

/// Phase-1 configuration, as a host would supply it (trusted config).
struct Config {
    fee_rate_bps: u64,
    accounts: Vec<(u64, u64)>,
}

fn configure() -> Config {
    Config {
        fee_rate_bps: 15,
        accounts: vec![(1, 500_000), (2, 0), (3, 250_000)],
    }
}

/// Proposals: (sender, recipient, amount).
fn proposals() -> Vec<(u64, u64, u64)> {
    vec![
        (1, 2, 100_000),
        (3, 2, 50_000),
        (2, 3, 500_000),
        (1, 3, 10_000),
    ]
}

fn main() -> std::io::Result<()> {
    let out: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "audit-bundle".to_string())
        .into();
    std::fs::create_dir_all(&out)?;

    let cfg = configure();
    let ledger = Ledger::new(&cfg.accounts, cfg.fee_rate_bps);
    let total_before = ledger.total().expect("no poisoned locks");

    let mut log = String::new();
    writeln!(log, "tpt-statera reference settlement").unwrap();
    writeln!(
        log,
        "fee_rate_bps={} accounts={:?}",
        cfg.fee_rate_bps, cfg.accounts
    )
    .unwrap();
    writeln!(log, "total_before={total_before}").unwrap();

    for (i, (from, to, amount)) in proposals().into_iter().enumerate() {
        let result = ledger.submit(from, to, amount).expect("accounts exist");
        let line = match result {
            Submission::Committed(ValidationOutcome::Allow {
                rounded_fee,
                dust_retained_by_sender,
                ..
            }) => {
                format!("committed fee={rounded_fee} dust_scaled={dust_retained_by_sender}")
            }
            Submission::Committed(ValidationOutcome::Deny) => "internal: committed a Deny".into(),
            Submission::Denied => "denied".into(),
            Submission::Stale => "stale".into(),
        };
        writeln!(log, "proposal {i}: {from}->{to} amount={amount}: {line}").unwrap();
    }

    let total_after = ledger.total().expect("no poisoned locks");
    writeln!(log, "total_after={total_after}").unwrap();
    writeln!(
        log,
        "conservation={}",
        if total_before == total_after {
            "PASS"
        } else {
            "FAIL"
        }
    )
    .unwrap();
    writeln!(log, "treasury={}", ledger.balance(TREASURY).unwrap()).unwrap();
    assert_eq!(total_before, total_after, "conservation violated");

    std::fs::write(out.join("manifest.txt"), &log)?;

    // CARGO_MANIFEST_DIR is crates/tpt-statera-runtime, so the repo root is two levels up.
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let proofs = repo.join("proofs");

    // Generate the certificate, if a telos binary is given (todo 4.02). This is the same generator CI
    // uses, so the bundle's certificate comes from the current proofs, not from an old commit.
    // Without TPT_TELOS the bundle copies whatever is committed, and the verdict below says so.
    match std::env::var("TPT_TELOS") {
        Ok(telos) => {
            let python = std::env::var("TPT_PYTHON").unwrap_or_else(|_| "python".to_string());
            let status = std::process::Command::new(python)
                .arg(repo.join("scripts/generate_certificate.py"))
                .args(["--telos", &telos, "--solver", "z3"])
                .status()?;
            if !status.success() {
                return Err(std::io::Error::other(format!(
                    "certificate generation failed (exit {status})"
                )));
            }
            writeln!(
                log,
                "certificate_source=generated (telos={telos}, solver=z3)"
            )
            .unwrap();
        }
        Err(_) => writeln!(log, "certificate_source=committed (TPT_TELOS not set)").unwrap(),
    }

    // Copy the proof artifacts into the bundle.
    let mut copied = Vec::new();
    for rel in [
        "certificate.json",
        "transcripts/transfer_validator_full.json",
    ] {
        let src = proofs.join(rel);
        if src.exists() {
            let dst = out.join(rel.replace('/', "_"));
            std::fs::copy(&src, &dst)?;
            copied.push(dst.display().to_string());
        }
    }
    // Record the certificate verdict next to the run log, so the bundle says what it contains.
    let cert_path = proofs.join("certificate.json");
    let verdict = match std::fs::read_to_string(&cert_path) {
        Ok(text) if text.contains("\"overall\": \"PASS\"") => "PASS".to_string(),
        Ok(_) => "NOT PASS".to_string(),
        Err(_) => "MISSING".to_string(),
    };
    writeln!(log, "certificate_overall={verdict}").unwrap();
    std::fs::write(out.join("manifest.txt"), &log)?;
    println!("{log}");
    println!("bundle written to {}", out.display());
    for c in copied {
        println!("  copied {c}");
    }
    Ok(())
}

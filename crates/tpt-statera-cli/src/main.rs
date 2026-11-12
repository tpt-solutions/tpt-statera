// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The `statera` command (todo 5C).
//!
//! Exit codes: `0` success or Allow, `1` a denial or a failed check, `2` a usage, file or parse error.
//! Every check runs the same library code as the crates it wraps. The CLI adds no validation logic.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Deserialize;
use tpt_statera_core::{validate_transfer_explained, DenyReason, SystemState, TransferProposal};
use tpt_statera_policy::{
    validate_with_policy, DustDestination, FeeShares, FeeTiers, RoundingMode, RoundingPolicy,
    Share, Tier,
};

#[derive(Parser)]
#[command(
    name = "statera",
    version,
    about = "Validate transfers, check fee policies and verify proof certificates."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate one transfer against a balance snapshot. Exit 0 on Allow; exit 1 on Deny, with the reason.
    Validate {
        /// Sender balance before the transfer.
        #[arg(long)]
        sender: u64,
        /// Recipient balance before the transfer.
        #[arg(long)]
        recipient: u64,
        /// Treasury balance before the transfer.
        #[arg(long, default_value_t = 0)]
        treasury: u64,
        /// Amount the recipient should receive, in whole units.
        #[arg(long)]
        amount: u64,
        /// Fee rate in basis points (0..=10000).
        #[arg(long = "fee-bps")]
        fee_bps: u64,
    },
    /// For each amount, show the tier rate, the fee and how the fee is divided under a policy file.
    SimulateFees {
        /// Policy file (TOML). See `crates/tpt-statera-cli/README.md`.
        #[arg(long)]
        policy: PathBuf,
        /// Sender balance to use for every row.
        #[arg(long, default_value_t = 1_000_000_000_000)]
        sender_balance: u64,
        /// Amounts to simulate, in whole units.
        #[arg(required = true)]
        amounts: Vec<u64>,
    },
    /// Work with fee policy files.
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    /// Work with proof certificates.
    Cert {
        #[command(subcommand)]
        command: CertCommand,
    },
}

#[derive(Subcommand)]
enum PolicyCommand {
    /// Check a policy file. Every section is checked, and each problem is reported. Exit 0 if valid.
    Lint {
        /// Policy file (TOML).
        path: PathBuf,
    },
}

#[derive(Subcommand)]
enum CertCommand {
    /// Verify a certificate against the files it names. Solver-free. Exit 0 if consistent.
    Verify {
        /// Path to the certificate JSON, for example `proofs/certificate.json`.
        path: PathBuf,
        /// Repository root that the paths inside the certificate are relative to.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

/// The policy file format. Unknown keys are refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyFile {
    rounding: RoundingSection,
    tiers: Vec<Tier>,
    split: SplitSection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoundingSection {
    mode: RoundingMode,
    dust: DustDestination,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitSection {
    /// The treasury's share, in basis points. The shares must sum to 10 000 with `others`.
    treasury_bps: u64,
    /// Other recipients. May be empty.
    #[serde(default)]
    others: Vec<Share>,
}

/// A policy that passed every check.
struct LoadedPolicy {
    tiers: FeeTiers,
    shares: FeeShares,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(code) => code,
        Err(msg) => {
            eprintln!("statera: {msg}");
            ExitCode::from(2)
        }
    }
}

fn run(command: Command) -> Result<ExitCode, String> {
    match command {
        Command::Validate {
            sender,
            recipient,
            treasury,
            amount,
            fee_bps,
        } => {
            let state = SystemState::new(sender, recipient, treasury);
            let proposal = TransferProposal::new(amount, fee_bps);
            match validate_transfer_explained(state, proposal) {
                Ok(outcome) => {
                    println!("{outcome}");
                    Ok(ExitCode::SUCCESS)
                }
                Err(reason) => {
                    println!("deny: {reason} ({})", reason.code());
                    Ok(ExitCode::from(1))
                }
            }
        }
        Command::SimulateFees {
            policy,
            sender_balance,
            amounts,
        } => {
            let loaded = load_policy(&policy)?;
            println!("amount\trate_bps\tfee\ttreasury\tparts");
            for amount in amounts {
                let state = SystemState::new(sender_balance, 0, 0);
                let d = validate_with_policy(state, amount, &loaded.tiers, &loaded.shares)
                    .map_err(|e| format!("amount {amount}: {e}"))?;
                match d.fee_split {
                    Some(split) => {
                        let fee = split.treasury + split.parts.iter().map(|(_, a)| a).sum::<u64>();
                        let parts = split
                            .parts
                            .iter()
                            .map(|(id, a)| format!("{id}={a}"))
                            .collect::<Vec<_>>()
                            .join(",");
                        println!(
                            "{amount}\t{}\t{fee}\t{}\t{parts}",
                            d.fee_rate_bps, split.treasury
                        );
                    }
                    None => println!(
                        "{amount}\t{}\tdenied: {}",
                        d.fee_rate_bps,
                        deny_reason(state, amount, d.fee_rate_bps)
                    ),
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Policy {
            command: PolicyCommand::Lint { path },
        } => lint_policy(&path),
        Command::Cert {
            command: CertCommand::Verify { path, root },
        } => verify_cert(&path, &root),
    }
}

fn deny_reason(state: SystemState, amount: u64, rate: u64) -> DenyReason {
    match validate_transfer_explained(state, TransferProposal::new(amount, rate)) {
        Err(reason) => reason,
        Ok(_) => DenyReason::Unclassified,
    }
}

fn read_policy_file(path: &Path) -> Result<PolicyFile, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Load and fully validate a policy file. Stops at the first problem.
fn load_policy(path: &Path) -> Result<LoadedPolicy, String> {
    let file = read_policy_file(path)?;
    RoundingPolicy::new(file.rounding.mode, file.rounding.dust).map_err(|e| e.to_string())?;
    let tiers = FeeTiers::new(file.tiers).map_err(|e| e.to_string())?;
    let shares =
        FeeShares::new(file.split.treasury_bps, file.split.others).map_err(|e| e.to_string())?;
    Ok(LoadedPolicy { tiers, shares })
}

/// Check each section and report every problem, not just the first.
fn lint_policy(path: &Path) -> Result<ExitCode, String> {
    let file = read_policy_file(path)?;
    let mut problems = Vec::new();
    if let Err(e) = RoundingPolicy::new(file.rounding.mode, file.rounding.dust) {
        problems.push(format!("rounding: {e}"));
    }
    if let Err(e) = FeeTiers::new(file.tiers) {
        problems.push(format!("tiers: {e}"));
    }
    if let Err(e) = FeeShares::new(file.split.treasury_bps, file.split.others) {
        problems.push(format!("split: {e}"));
    }
    if problems.is_empty() {
        println!("{}: valid", path.display());
        Ok(ExitCode::SUCCESS)
    } else {
        for p in &problems {
            println!("{}: {p}", path.display());
        }
        Ok(ExitCode::from(1))
    }
}

fn verify_cert(path: &Path, root: &Path) -> Result<ExitCode, String> {
    let report =
        tpt_statera_proof::verify_certificate_file(path, root).map_err(|e| e.to_string())?;
    println!("{report}");
    Ok(if report.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Cross-process client for `ledger_server` (todo 2B.05).
//!
//! Run: `cargo run -p tpt-statera-runtime --example ledger_client -- 127.0.0.1:7878 <from> <to> <amount>`
//! Without arguments it sends a short fixed batch, so several clients can run at once.

use tpt_statera_runtime::service::{Reply, TcpClient};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let addr = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:7878".to_string());
    let mut client = TcpClient::connect(&addr).expect("connect to ledger_server");

    let requests: Vec<(u64, u64, u64)> = if args.len() >= 5 {
        vec![(
            args[2].parse().unwrap(),
            args[3].parse().unwrap(),
            args[4].parse().unwrap(),
        )]
    } else {
        // Fixed batch: 1 pays 2 and 3 in turn.
        (0..20).map(|i| (1, 2 + (i % 2), 1_000 + i)).collect()
    };

    for (from, to, amount) in requests {
        match client.submit(from, to, amount) {
            Ok(Reply::Committed { fee }) => println!("{from}->{to} {amount}: committed, fee {fee}"),
            Ok(Reply::Denied) => println!("{from}->{to} {amount}: denied"),
            Ok(Reply::Stale) => println!("{from}->{to} {amount}: stale, resubmit"),
            Err(e) => println!("{from}->{to} {amount}: error {e:?}"),
        }
    }
}

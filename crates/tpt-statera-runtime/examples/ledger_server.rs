// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Cross-process single-writer server (todo 2B.05). Bind to loopback.
//!
//! Run: `cargo run -p tpt-statera-runtime --example ledger_server -- 127.0.0.1:7878`
//! Then, from other processes: `cargo run -p tpt-statera-runtime --example ledger_client -- 127.0.0.1:7878`
//!
//! The server starts with accounts 1 (funded with 1 000 000), 2 and 3, and a 15 bps fee.
//! Authentication: set TPT_LEDGER_TOKEN on the server and on each client. Without a token the server
//! only accepts loopback addresses.

use std::net::TcpListener;

use tpt_statera_runtime::service::{serve_tcp, WriterService};
use tpt_statera_runtime::Ledger;

fn main() -> std::io::Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7878".to_string());
    let listener = TcpListener::bind(&addr)?;
    println!("ledger_server listening on {}", listener.local_addr()?);
    let ledger = Ledger::new(&[(1, 1_000_000), (2, 0), (3, 0)], 15);
    let (svc, _writer) = WriterService::spawn(ledger);
    // The token is read from TPT_LEDGER_TOKEN. Without it the server refuses non-loopback addresses.
    let token = std::env::var("TPT_LEDGER_TOKEN").ok();
    serve_tcp(listener, svc, token)
}

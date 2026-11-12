// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Fuzz a sequence of `Ledger::submit` calls. The total of all balances must be unchanged after every call.
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_statera_runtime::{Ledger, Submission};

fuzz_target!(|data: &[u8]| {
    // Three accounts, one fee rate taken from the first byte pair, then 5-byte operations.
    let fee_rate_bps = u16::from_le_bytes([*data.first().unwrap_or(&0), *data.get(1).unwrap_or(&0)]) as u64 % 10_001;
    let ledger = Ledger::new(&[(1, 1_000_000), (2, 500), (3, 0)], fee_rate_bps);
    let before = ledger.total().unwrap();

    for op in data.get(2..).unwrap_or(&[]).chunks(5) {
        if op.len() < 5 {
            break;
        }
        let sender = (op[0] % 4) as u64; // 0 is the treasury, which is also a valid endpoint
        let recipient = (op[1] % 4) as u64;
        let amount = u32::from_le_bytes([op[2], op[3], op[4], 0]) as u64;
        match ledger.submit(sender, recipient, amount) {
            Ok(Submission::Committed(_)) | Ok(Submission::Denied(_)) | Ok(Submission::Stale) => {}
            Err(_) => {} // unknown account or self-transfer: an error, with no state change
        }
        assert_eq!(ledger.total().unwrap(), before, "conservation broken");
    }
});

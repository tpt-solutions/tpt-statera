// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Per-call FM-elimination latency for the 10-party netting settlement (todo 1E.02).
//!
//! One "call" = discharge every obligation of `settle10` in `proofs/netting_10party.telos`:
//! the conservation postcondition, and the non-negativity postcondition for each of the
//! ten parties, from the premises (zero-sum requires and per-party solvency requires).
//! Latency is measured with `Instant` around the solver calls only (no parsing, no I/O).

use std::time::Instant;
use tpt_telos_ir::{Constraint, Linear, Relation};
use tpt_telos_verifier::solver::entails;

const N: usize = 10;

fn premises() -> Vec<Constraint> {
    let mut cs = Vec::new();
    // Zero-sum: d0 + ... + d9 == 0
    let mut sum = Linear::constant_only(0);
    for i in 0..N {
        sum = sum.add(&Linear::var(&format!("d{i}")));
    }
    cs.push(Constraint(sum, Relation::Eq));
    // Per-party solvency: b_i + d_i >= 0
    for i in 0..N {
        let e = Linear::var(&format!("b{i}")).add(&Linear::var(&format!("d{i}")));
        cs.push(Constraint(e, Relation::Ge));
    }
    // Pre-state non-negativity, as the Account invariant provides: b_i >= 0
    for i in 0..N {
        cs.push(Constraint(Linear::var(&format!("b{i}")), Relation::Ge));
    }
    cs
}

/// All obligations for one settlement, as conclusions to entail from `premises`.
fn obligations() -> Vec<Constraint> {
    let mut out = Vec::new();
    // Conservation: sum(b_i + d_i) - sum(b_i) == 0, i.e. sum(d_i) == 0.
    let mut cons = Linear::constant_only(0);
    for i in 0..N {
        cons = cons.add(&Linear::var(&format!("d{i}")));
    }
    out.push(Constraint(cons, Relation::Eq));
    // Non-negativity of each post-state balance: b_i + d_i >= 0.
    for i in 0..N {
        let e = Linear::var(&format!("b{i}")).add(&Linear::var(&format!("d{i}")));
        out.push(Constraint(e, Relation::Ge));
    }
    out
}

/// Policy augmentation (todo 2A.05): a fee split among two recipients and the treasury at fixed shares
/// 3333 / 3333 / 3334 bps (docs/POLICY_MODEL.md D2). With constant shares every constraint is linear, so the
/// Fourier–Motzkin solver can run it. The floor relation for each share is the premise, and the remainder
/// bound is the obligation. The netting set is kept, so the augmented set is netting plus the split.
fn policy_premises() -> Vec<Constraint> {
    // Shares, in basis points. They sum to 10 000.
    const SHARES: [(&str, i64); 3] = [("p1", 3333), ("p2", 3333), ("pt", 3334)];
    let fee = Linear::var("fee");
    let mut cs = vec![Constraint(fee.clone(), Relation::Ge)];
    for (name, bps) in SHARES {
        let p = Linear::var(name);
        // p * 10000 <= fee * bps          <=>   fee*bps - p*10000 >= 0
        cs.push(Constraint(fee.scale(bps).sub(&p.scale(10_000)), Relation::Ge));
        // fee * bps < p * 10000 + 10000   <=>   p*10000 + 9999 - fee*bps >= 0 (integers)
        cs.push(Constraint(
            p.scale(10_000).add(&Linear::constant_only(9_999)).sub(&fee.scale(bps)),
            Relation::Ge,
        ));
        // each part is non-negative
        cs.push(Constraint(p, Relation::Ge));
    }
    cs
}

/// Obligations for the split: the remainder fee - (p1 + p2 + pt) is in [0, 3).
fn policy_obligations() -> Vec<Constraint> {
    let rem = Linear::var("fee")
        .sub(&Linear::var("p1"))
        .sub(&Linear::var("p2"))
        .sub(&Linear::var("pt"));
    vec![
        Constraint(rem.clone(), Relation::Ge),
        Constraint(Linear::constant_only(2).sub(&rem), Relation::Ge),
    ]
}

fn one_call(prem: &[Constraint], obl: &[Constraint]) -> bool {
    obl.iter().all(|c| entails(prem, c))
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() as f64) * p).ceil() as usize - 1;
    sorted[idx.min(sorted.len() - 1)]
}

fn main() {
    let calls: usize = std::env::var("FM_CALLS").ok().and_then(|s| s.parse().ok()).unwrap_or(1000);
    let warmup: usize = 50;
    // FM_AUGMENT=policy adds the fee split (todo 2A.05). Unset, the benchmark is the netting set as before.
    let augment = std::env::var("FM_AUGMENT").unwrap_or_default();
    let mut prem = premises();
    let mut obl = obligations();
    if augment == "policy" {
        prem.extend(policy_premises());
        obl.extend(policy_obligations());
    }
    println!("augment={}", if augment.is_empty() { "none" } else { augment.as_str() });

    // Sanity: the obligations must be entailed, otherwise the timings measure a failure.
    assert!(one_call(&prem, &obl), "netting obligations not entailed; benchmark invalid");

    for _ in 0..warmup {
        std::hint::black_box(one_call(&prem, &obl));
    }
    let mut samples_ms = Vec::with_capacity(calls);
    for _ in 0..calls {
        let t = Instant::now();
        let ok = one_call(std::hint::black_box(&prem), std::hint::black_box(&obl));
        let dt = t.elapsed().as_secs_f64() * 1000.0;
        assert!(ok);
        samples_ms.push(dt);
    }
    if let Ok(path) = std::env::var("FM_RAW_CSV") {
        let mut out = String::from("call_index,latency_ms
");
        for (i, v) in samples_ms.iter().enumerate() {
            out.push_str(&format!("{i},{v:.6}
"));
        }
        std::fs::write(&path, out).expect("write raw csv");
        println!("raw_csv={path}");
    }
    samples_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = percentile(&samples_ms, 0.5);
    let p99 = percentile(&samples_ms, 0.99);
    let mean = samples_ms.iter().sum::<f64>() / samples_ms.len() as f64;
    println!("fm-netting-bench: 10-party settlement, {} obligations per call", obl.len());
    println!("calls={calls} warmup={warmup}");
    println!("median_ms={median:.6}");
    println!("p99_ms={p99:.6}");
    println!("mean_ms={mean:.6}");
    println!("min_ms={:.6} max_ms={:.6}", samples_ms[0], samples_ms[samples_ms.len() - 1]);
    let gate = median < 10.0 && p99 < 50.0;
    println!("gate(median<10ms && p99<50ms)={}", if gate { "PASS" } else { "FAIL" });
}

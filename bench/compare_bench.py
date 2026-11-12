# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions
"""Regression check for the FM netting benchmark (todo 1E.04).

Compares a new run's summary against a committed baseline. Exits 1 when the median
degrades by more than the threshold (default 20%, as the spec's example). Also exits 1 if
the new run's own gate line is FAIL or missing.

Usage:
    python bench/compare_bench.py --baseline bench/results/fm_netting_summary.txt \
        --new <new-summary.txt> [--threshold 0.20]

Summary format is the output of `bench/fm_netting` (key=value lines).
"""

from __future__ import annotations

import argparse
import pathlib
import sys


def parse(path: pathlib.Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            k, v = line.split("=", 1)
            out[k.strip()] = v.strip()
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", required=True, type=pathlib.Path)
    ap.add_argument("--new", required=True, type=pathlib.Path, action="append")
    ap.add_argument("--threshold", type=float, default=0.20)
    args = ap.parse_args()

    base = parse(args.baseline)
    runs = [parse(p) for p in args.new]
    for r in runs:
        if "gate(median<10ms && p99<50ms)" not in r:
            print("ALERT: a new run has no gate line; treating as failure")
            return 1
        if r["gate(median<10ms && p99<50ms)"] != "PASS":
            print("ALERT: a new run's absolute gate is FAIL")
            return 1

    b_med = float(base["median_ms"])
    medians = sorted(float(r["median_ms"]) for r in runs)
    n_med = medians[len(medians) // 2]
    change = (n_med - b_med) / b_med
    print(f"baseline median {b_med:.6f} ms; new run medians {medians}; median-of-runs {n_med:.6f} ms; change {change:+.1%}")
    if change > args.threshold:
        print(f"ALERT: median degraded by more than {args.threshold:.0%}. Open an issue (todo 1E.04).")
        return 1
    print("OK: within threshold")
    return 0


if __name__ == "__main__":
    sys.exit(main())

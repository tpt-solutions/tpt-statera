#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions
#
# todo 1A.06 / 1F.02 / 1A.07c: verify every proof model and every negative control.
#
#   * each proofs/*.telos must exit 0 (telos prints per-path results; a disjunction group
#     prints [FAIL] for its alternative branches, so the exit code is the verdict, not a grep)
#   * each proofs/negative/*.telos must exit NON-zero. A control that passes means the
#     solver accepted a false claim, and the run fails.
#
# Usage:  python scripts/verify_proofs.py --telos path/to/telos [--solver z3]
# Exit status is 0 only if every proof passes and every control fails.

import argparse
import glob
import os
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))


TRANSCRIPTS = os.path.join(ROOT, "proofs", "transcripts", "verify")


def run(telos, solver, path):
    cmd = [telos, "verify", "--solver", solver, path]
    proc = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
    out = proc.stdout + proc.stderr
    # Keep the raw solver output as evidence (todo 1C.03). Overwritten on each run.
    os.makedirs(TRANSCRIPTS, exist_ok=True)
    tag = os.path.splitext(os.path.basename(path))[0]
    if os.path.basename(os.path.dirname(path)) == "negative":
        tag = "negative." + tag
    with open(os.path.join(TRANSCRIPTS, f"{tag}.{solver}.txt"), "w", newline="\n") as f:
        f.write(out)
    return proc.returncode, out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--telos", required=True, help="path to the telos binary built with --features z3")
    ap.add_argument("--solver", default="z3", choices=["z3", "fourier-motzkin"])
    args = ap.parse_args()

    proofs = sorted(glob.glob(os.path.join(ROOT, "proofs", "*.telos")))
    controls = sorted(glob.glob(os.path.join(ROOT, "proofs", "negative", "*.telos")))
    if not proofs or not controls:
        print("error: no proofs or no negative controls found", file=sys.stderr)
        return 2

    failed = []
    for path in proofs:
        code, out = run(args.telos, args.solver, path)
        name = os.path.relpath(path, ROOT)
        verdict = "PASS" if code == 0 else "FAIL"
        print(f"[{verdict}] proof    {name} (exit {code})")
        if code != 0:
            failed.append(name)
            print("\n".join(out.strip().splitlines()[-5:]))

    for path in controls:
        code, out = run(args.telos, args.solver, path)
        name = os.path.relpath(path, ROOT)
        # A control must fail *because a claim is refuted*, not because it does not parse.
        ok = code != 0 and "[FAIL]" in out
        print(f"[{'OK' if ok else 'UNSOUND'}] control  {name} (exit {code}, must be non-zero)")
        if not ok:
            failed.append(name)

    if failed:
        print(f"\n{len(failed)} check(s) failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"\nall {len(proofs)} proofs pass and all {len(controls)} negative controls fail")
    return 0


if __name__ == "__main__":
    sys.exit(main())

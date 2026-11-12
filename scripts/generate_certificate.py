# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions
"""Generate the first-draft Proof Certificate (todo 1F.02, EC3).

Runs `telos verify --json` on the certificate-scoped proof files, stores each raw
solver transcript under proofs/transcripts/, and writes a deterministic
certificate JSON. Standard library only, so it runs anywhere CI has Python 3.

Usage (from repo root):
    python scripts/generate_certificate.py --telos path/to/telos.exe

Certificate scope is deliberately limited to the transfer proofs. Multi-party
netting is excluded by the spec §7 caveat (todo 1F.03) and is not certified here.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
CERT_PROOFS = ["proofs/transfer_validator_full.telos"]
# The full model needs exact monomials and the fixed disjunction lowering, so it runs under Z3.
SOLVER_FLAG = {"fourier-motzkin": [], "z3": ["--solver", "z3"]}
OUT_PATH = REPO / "proofs" / "certificate.json"
TRANSCRIPT_DIR = REPO / "proofs" / "transcripts"
SCHEMA = "tpt-statera.proof-certificate/0.1-draft"

CAVEAT = (
    "Phase 1's \"first-draft Proof Certificate\" is an illustrative artifact demonstrating the certificate format and workflow. It is not, by itself, a compliance claim, does not cover concurrent-access or multi-party netting scenarios (Section 8), and should not be represented to customers or auditors as a completed regulatory mapping until Phase 2 is delivered and Phase 3's independent audit has occurred."
)

# Control mappings are stubs by design (todo 1F.04).
CONTROL_STUBS = {
    "SOC2 CC6.1 (Logical Access Security)": "mapping pending Phase 2",
    "MiCA Article 51 (Asset safeguarding)": "mapping pending Phase 2",
}


def sha256_file(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def run_telos(telos: str, rel_path: str, solver: str) -> dict:
    proc = subprocess.run(
        [telos, "verify", rel_path, "--json", *SOLVER_FLAG[solver]],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    )
    if not proc.stdout.strip():
        raise SystemExit(f"telos produced no output for {rel_path}: {proc.stderr}")
    return json.loads(proc.stdout)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--telos", required=True, help="path to telos binary")
    ap.add_argument("--solver", choices=sorted(SOLVER_FLAG), default="z3",
                    help="solver for the proofs (default z3; the full model needs it)")
    args = ap.parse_args()

    TRANSCRIPT_DIR.mkdir(parents=True, exist_ok=True)
    claims = []
    inputs = []
    all_passed = True

    for rel in CERT_PROOFS:
        src = REPO / rel
        digest = sha256_file(src)
        short = digest[:8]
        inputs.append({"path": rel, "sha256": digest})

        transcript = run_telos(args.telos, rel, args.solver)
        t_name = pathlib.Path(rel).stem + ".json"
        (TRANSCRIPT_DIR / t_name).write_text(
            json.dumps(transcript, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        all_passed &= bool(transcript.get("passed"))

        # The solver reports each obligation once per control-flow path, and names the path
        # variants `function[branch k]`. A claim is one obligation: an independent check, or a
        # disjunction group. Each claim passes only if it passes on every path (AND across paths).
        # A disjunction group passes when one of its DNF branches holds (see telos CLI JSON).
        merged = {}
        for fn in transcript.get("functions", []):
            fname = re.sub(r"\[branch \d+\]$", "", fn.get("func_name") or "")
            for chk in fn.get("checks", []):
                if chk.get("or_group") is not None:
                    continue  # reported through the group verdict below
                key = (rel, fname, chk.get("location", {}).get("line"), chk.get("description"), "check")
                entry = merged.setdefault(key, {"passed": True, "instances": 0})
                entry["instances"] += 1
                entry["passed"] &= bool(chk.get("passed"))
            for grp in fn.get("groups", []):
                key = (rel, fname, grp.get("location", {}).get("line"), grp.get("description"), "group")
                entry = merged.setdefault(key, {"passed": True, "instances": 0})
                entry["instances"] += 1
                entry["passed"] &= bool(grp.get("passed"))
        # Sorted so the certificate does not depend on check order.
        for idx, key in enumerate(sorted(merged, key=lambda k: (k[0], k[1], k[2] or 0, k[3] or "", k[4])), start=1):
            src, fname, line, desc, kind = key
            claims.append(
                {
                    "proof_id": f"TSC-{short}-{idx:03d}",
                    "source": src,
                    "function": fname,
                    "line": line,
                    "description": desc,
                    "kind": "disjunction" if kind == "group" else "obligation",
                    "result": "PASS" if merged[key]["passed"] else "FAIL",
                    "path_instances": merged[key]["instances"],
                    "transcript": f"proofs/transcripts/{t_name}",
                }
            )

    # Strip any absolute path that leaked into a transcript so output stays portable.
    for t in TRANSCRIPT_DIR.glob("*.json"):
        text = t.read_text(encoding="utf-8")
        text = re.sub(r'"file": "[^"]*/', '"file": "', text)
        t.write_text(text, encoding="utf-8")

    cert = {
        "schema": SCHEMA,
        "status": "ILLUSTRATIVE",
        "caveat": CAVEAT,
        "scope": "validate_transfer: all clauses (conservation, no-overdraft, no-op denial, both rounding bounds, overflow denial, restated denial) with the exact Floor product, checked by " + args.solver + "; not multi-party netting",
        "toolchain": {"solver": "tpt-telos verify --solver " + args.solver},
        "inputs": inputs,
        "claims": claims,
        "overall": "PASS" if all_passed else "FAIL",
        "control_mappings": CONTROL_STUBS,
    }
    OUT_PATH.write_text(json.dumps(cert, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUT_PATH.relative_to(REPO)}: {len(claims)} claims, overall {cert['overall']}")
    return 0 if all_passed else 1


if __name__ == "__main__":
    sys.exit(main())

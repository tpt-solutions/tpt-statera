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
CERT_PROOFS = ["proofs/transfer_validator.telos"]
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


def run_telos(telos: str, rel_path: str) -> dict:
    proc = subprocess.run(
        [telos, "verify", rel_path, "--json"],
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

        transcript = run_telos(args.telos, rel)
        t_name = pathlib.Path(rel).stem + ".json"
        (TRANSCRIPT_DIR / t_name).write_text(
            json.dumps(transcript, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        all_passed &= bool(transcript.get("passed"))

        # The solver reports each obligation once per control-flow path. Collapse
        # identical (function, line, description) entries into one claim. A claim
        # passes only if every path instance passes.
        merged = {}
        order = []
        for fn in transcript.get("functions", []):
            for chk in fn.get("checks", []):
                # Path variants appear as "validate[branch k]"; normalise the name.
                fname = re.sub(r"\[branch \d+\]$", "", fn.get("func_name") or "")
                key = (fname, chk.get("location", {}).get("line"), chk.get("description"))
                if key not in merged:
                    merged[key] = {"passed": True, "instances": 0}
                    order.append(key)
                merged[key]["instances"] += 1
                merged[key]["passed"] &= bool(chk.get("passed"))
        for idx, key in enumerate(order, start=1):
            fname, line, desc = key
            claims.append(
                {
                    "proof_id": f"TSC-{short}-{idx:03d}",
                    "source": rel,
                    "function": fname,
                    "line": line,
                    "description": desc,
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
        "scope": "validate_transfer: linear conservation core under the Floor-fee specification; not multi-party netting; disjunctive clauses excluded (see proofs/README.md)",
        "toolchain": {"solver": "tpt-telos verify (built-in Fourier-Motzkin)"},
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

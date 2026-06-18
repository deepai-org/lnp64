#!/usr/bin/env python3
"""Self-test the formal proof manifest checker failure modes."""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_formal_proof_manifest.py"
MANIFEST = ROOT / "formal/proof_obligations_manifest.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_checker(manifest: Path, proof_gate: Path | None = None) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_PROOF_MANIFEST"] = str(manifest)
    if proof_gate is not None:
        env["LNP64_PROOF_GATE"] = str(proof_gate)
    result = subprocess.run(
        [sys.executable, str(CHECKER)],
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    result.stdout = (result.stdout or "") + (result.stderr or "")
    return result


def expect_failure(manifest: Path, expected: str, proof_gate: Path | None = None) -> None:
    result = run_checker(manifest, proof_gate)
    require(result.returncode != 0, f"expected checker failure for {manifest}")
    require(expected in result.stdout, f"checker failure did not include {expected!r}: {result.stdout}")


def main() -> None:
    base = json.loads(MANIFEST.read_text(encoding="utf-8"))
    valid = run_checker(MANIFEST)
    require(valid.returncode == 0, f"current proof manifest failed: {valid.stdout}")
    require("formal proof manifest ok" in valid.stdout, "current proof manifest did not print success")

    with tempfile.TemporaryDirectory(prefix="lnp64-proof-manifest-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)

        missing_required = copy.deepcopy(base)
        for obligation in missing_required["obligations"]:
            if obligation["id"] == "A8":
                obligation["theorems"].remove("m2_signal_compatibility_cannot_create_authority_or_bypass_masks")
                break
        missing_required_path = tmp / "missing_required.json"
        write_json(missing_required_path, missing_required)
        expect_failure(missing_required_path, "A8: missing required roadmap theorem")

        placeholder_manifest = copy.deepcopy(base)
        s0_obligation = placeholder_manifest["obligations"][0]
        placeholder_lean = tmp / "S0Placeholder.lean"
        placeholder_lean.write_text(
            "\n".join(
                [f"theorem {theorem} : True := by trivial" for theorem in s0_obligation["theorems"]]
                + ["axiom forbidden_placeholder : True", ""]
            ),
            encoding="utf-8",
        )
        s0_obligation["lean_file"] = str(placeholder_lean)
        placeholder_manifest_path = tmp / "placeholder.json"
        write_json(placeholder_manifest_path, placeholder_manifest)

        proof_gate = tmp / "proof_gate.sh"
        proof_gate.write_text(
            "\n".join([str(placeholder_lean)] + [obligation["lean_file"] for obligation in base["obligations"][1:]]) + "\n",
            encoding="utf-8",
        )
        expect_failure(placeholder_manifest_path, "S0: forbidden Lean placeholder", proof_gate)

    print("formal proof manifest checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Self-test RTL co-simulation manifest checker failure modes."""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_cosim_manifest.py"
MANIFEST = ROOT / "tests/traces/rtl_cosim_manifest.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def run_checker(manifest: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_COSIM_MANIFEST"] = str(manifest)
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


def expect_failure(manifest: Path, expected: str) -> None:
    result = run_checker(manifest)
    require(result.returncode != 0, f"expected checker failure for {manifest}")
    require(expected in result.stdout, f"checker failure did not include {expected!r}: {result.stdout}")


def gate_by_name(manifest: dict, name: str) -> dict:
    for gate in manifest["fixed_trace_gates"]:
        if gate["name"] == name:
            return gate
    raise SystemExit(f"missing fixed trace gate {name}")


def random_gate_by_name(manifest: dict, name: str) -> dict:
    for gate in manifest["bounded_random_gates"]:
        if gate["name"] == name:
            return gate
    raise SystemExit(f"missing random trace gate {name}")


def main() -> None:
    base = json.loads(MANIFEST.read_text(encoding="utf-8"))
    valid = run_checker(MANIFEST)
    require(valid.returncode == 0, f"current co-sim manifest failed: {valid.stdout}")
    require("rtl cosim manifest ok" in valid.stdout, "current co-sim manifest did not print success")

    with tempfile.TemporaryDirectory(prefix="lnp64-cosim-manifest-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)

        missing_m2_marker = copy.deepcopy(base)
        gate_by_name(missing_m2_marker, "m2_gate")["required_trace_markers"] = [
            "TRACE missing_signal_compat"
        ]
        missing_m2_path = tmp / "missing_m2_marker.json"
        write_json(missing_m2_path, missing_m2_marker)
        expect_failure(missing_m2_path, "missing required trace marker TRACE missing_signal_compat")

        missing_m5_marker = copy.deepcopy(base)
        gate_by_name(missing_m5_marker, "m5_dma")["required_trace_markers"] = [
            "TRACE missing_dma_pin"
        ]
        missing_m5_path = tmp / "missing_m5_marker.json"
        write_json(missing_m5_path, missing_m5_marker)
        expect_failure(missing_m5_path, "missing required trace marker TRACE missing_dma_pin")

        missing_timer_variation = copy.deepcopy(base)
        random_gate_by_name(missing_timer_variation, "m7_futex_atomic_seeded")["varies"] = [
            field
            for field in random_gate_by_name(missing_timer_variation, "m7_futex_atomic_seeded")["varies"]
            if field != "timer_deadline"
        ]
        missing_timer_path = tmp / "missing_timer_variation.json"
        write_json(missing_timer_path, missing_timer_variation)
        expect_failure(missing_timer_path, "m7_futex_atomic_seeded must vary timer_deadline")

    print("rtl cosim manifest checker self-test ok")


if __name__ == "__main__":
    main()

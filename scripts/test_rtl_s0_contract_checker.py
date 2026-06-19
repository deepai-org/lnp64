#!/usr/bin/env python3
"""Self-test S0 RTL contract checker failure modes."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_s0_contract.py"
FILELIST = ROOT / "tests/rtl/s0_filelist.f"
EXTRA_FILES = (
    "tests/rtl/s0_filelist.f",
    "scripts/run_rtl_s0.sh",
    "scripts/rtl_verilator_common.sh",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def run_checker(root: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_S0_CONTRACT_ROOT"] = str(root)
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


def filelist_entries() -> list[str]:
    entries: list[str] = []
    for raw in FILELIST.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line and not line.startswith("#"):
            entries.append(line)
    return entries


def copy_s0_tree(dst: Path) -> None:
    for relative in [*filelist_entries(), *EXTRA_FILES]:
        source = ROOT / relative
        target = dst / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)


def replace(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    require(old in text, f"{path} did not contain {old}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def expect_failure(root: Path, expected: str) -> None:
    result = run_checker(root)
    require(result.returncode != 0, f"expected checker failure for {root}")
    require(expected in result.stdout, f"checker failure did not include {expected!r}: {result.stdout}")


def main() -> None:
    valid = run_checker(ROOT)
    require(valid.returncode == 0, f"current S0 contract failed: {valid.stdout}")
    require("rtl s0 contract ok" in valid.stdout, "current S0 contract did not print success")

    with tempfile.TemporaryDirectory(prefix="lnp64-s0-contract-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)

        missing_record_field = tmp / "missing-record-field"
        copy_s0_tree(missing_record_field)
        replace(missing_record_field / "rtl/include/lnp64_pkg.sv", "completion_target", "completion_target_removed")
        expect_failure(missing_record_field, "lnp64_cmd_t missing fields: completion_target")

        missing_acceptance_marker = tmp / "missing-acceptance-marker"
        copy_s0_tree(missing_acceptance_marker)
        replace(
            missing_acceptance_marker / "rtl/sim/lnp64_s0_tb.sv",
            "raw physical interrupt/address/DMA/device authority became visible",
            "raw authority marker removed",
        )
        expect_failure(
            missing_acceptance_marker,
            "S0 acceptance testbench is missing marker: raw physical interrupt/address/DMA/device authority became visible",
        )

        missing_gate_marker = tmp / "missing-gate-marker"
        copy_s0_tree(missing_gate_marker)
        replace(missing_gate_marker / "scripts/run_rtl_s0.sh", "grep -q \"LNP64-RTL-S0 PASS\"", "grep -q \"MISSING-S0-PASS\"")
        expect_failure(missing_gate_marker, "S0 gate script is missing marker: grep -q \"LNP64-RTL-S0 PASS\"")

    print("rtl s0 contract checker self-test ok")


if __name__ == "__main__":
    main()

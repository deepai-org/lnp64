#!/usr/bin/env python3
"""Self-test Dockerfile-backed RTL command path checker failure modes."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_dockerfiles.py"
COPIED_FILES = (
    "Dockerfile.rtl-exec",
    "Dockerfile.rtl-proof",
    "Dockerfile.rtl-synth",
    "Dockerfile.rtl-board",
    "README.md",
    "scripts/run_all_gates.sh",
    "scripts/run_formal_rtl_roadmap_audit.sh",
    "scripts/run_rtl_execution_fast.sh",
    "scripts/run_rtl_execution_fast_docker.sh",
    "scripts/run_rtl_m1_refinement_docker.sh",
    "scripts/run_rtl_m1_refinement_gate.sh",
    "scripts/run_rtl_proof_docker.sh",
    "scripts/run_rtl_proof_gates.sh",
    "scripts/run_rtl_synth_docker.sh",
    "scripts/run_rtl_board_docker.sh",
    "scripts/run_rtl_board_ice40_s0.sh",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def run_checker(root: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_DOCKERFILES_ROOT"] = str(root)
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


def copy_tree(dst: Path) -> None:
    for relative in COPIED_FILES:
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
    require(valid.returncode == 0, f"current Dockerfile checker failed: {valid.stdout}")
    require("rtl Dockerfile command paths ok" in valid.stdout, "current Dockerfile checker did not print success")

    with tempfile.TemporaryDirectory(prefix="lnp64-dockerfile-checker-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)

        missing_board_package = tmp / "missing-board-package"
        copy_tree(missing_board_package)
        replace(missing_board_package / "Dockerfile.rtl-board", "python3-serial \\", "# python3-serial removed \\")
        expect_failure(missing_board_package, "Dockerfile.rtl-board: missing package python3-serial")

        missing_board_device = tmp / "missing-board-device"
        copy_tree(missing_board_device)
        replace(
            missing_board_device / "scripts/run_rtl_board_docker.sh",
            '--device "$resolved_uart:$resolved_uart"',
            "# missing device passthrough",
        )
        expect_failure(missing_board_device, "scripts/run_rtl_board_docker.sh: missing --device")

        missing_proof_rerun = tmp / "missing-proof-rerun"
        copy_tree(missing_proof_rerun)
        replace(
            missing_proof_rerun / "scripts/run_rtl_proof_docker.sh",
            "bash scripts/run_rtl_proof_gates.sh",
            "bash scripts/not_the_proof_gate.sh",
        )
        expect_failure(missing_proof_rerun, "scripts/run_rtl_proof_docker.sh: missing bash scripts/run_rtl_proof_gates.sh")

    print("rtl Dockerfile checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Self-test Track D FPGA bring-up manifest checker failure modes."""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_fpga_bringup_manifest.py"
MANIFEST = ROOT / "fpga/bringup/lnp64_track_d_bringup.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def run_checker(manifest: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_FPGA_BRINGUP_MANIFEST"] = str(manifest)
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


def step_by_id(manifest: dict, step_id: int) -> dict:
    for step in manifest["steps"]:
        if step["id"] == step_id:
            return step
    raise SystemExit(f"missing Track D step {step_id}")


def main() -> None:
    base = json.loads(MANIFEST.read_text(encoding="utf-8"))
    valid = run_checker(MANIFEST)
    require(valid.returncode == 0, f"current FPGA bring-up manifest failed: {valid.stdout}")
    require("fpga bringup manifest ok" in valid.stdout, "current FPGA bring-up manifest did not print success")

    with tempfile.TemporaryDirectory(prefix="lnp64-fpga-bringup-manifest-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)

        missing_step = copy.deepcopy(base)
        missing_step["steps"] = [step for step in missing_step["steps"] if step["id"] != 17]
        missing_step_path = tmp / "missing_step.json"
        write_json(missing_step_path, missing_step)
        expect_failure(missing_step_path, "Track D step ids must be")

        wrong_step_sources = copy.deepcopy(base)
        step_by_id(wrong_step_sources, 11)["filelists"] = ["tests/rtl/m1_filelist.f"]
        wrong_sources_path = tmp / "wrong_step_sources.json"
        write_json(wrong_sources_path, wrong_step_sources)
        expect_failure(wrong_sources_path, "step 11 missing smoke marker TRACE dma_copy")

    print("fpga bringup manifest checker self-test ok")


if __name__ == "__main__":
    main()

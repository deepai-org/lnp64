#!/usr/bin/env python3
"""Self-test Track B RTL manifest checker failure modes."""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_track_b_manifest.py"
MANIFEST = ROOT / "rtl/track_b_blocks_manifest.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def run_checker(manifest: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_TRACK_B_MANIFEST"] = str(manifest)
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


def block_by_id(manifest: dict, block_id: str) -> dict:
    for block in manifest["blocks"]:
        if block["id"] == block_id:
            return block
    raise SystemExit(f"missing Track B block {block_id}")


def main() -> None:
    base = json.loads(MANIFEST.read_text(encoding="utf-8"))
    valid = run_checker(MANIFEST)
    require(valid.returncode == 0, f"current Track B manifest failed: {valid.stdout}")
    require("rtl Track B manifest ok" in valid.stdout, "current Track B manifest did not print success")

    with tempfile.TemporaryDirectory(prefix="lnp64-track-b-manifest-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)

        missing_b0_module = copy.deepcopy(base)
        block_by_id(missing_b0_module, "B0")["modules"].remove("lnp64_uart")
        missing_b0_path = tmp / "missing_b0_module.json"
        write_json(missing_b0_path, missing_b0_module)
        expect_failure(missing_b0_path, "B0 manifest omits required skeleton modules")

        wrong_b6_sources = copy.deepcopy(base)
        block_by_id(wrong_b6_sources, "B6")["filelists"] = ["tests/rtl/m1_filelist.f"]
        wrong_b6_path = tmp / "wrong_b6_sources.json"
        write_json(wrong_b6_path, wrong_b6_sources)
        expect_failure(wrong_b6_path, "B6: missing roadmap run marker TRACE gate_call")

    print("rtl Track B manifest checker self-test ok")


if __name__ == "__main__":
    main()

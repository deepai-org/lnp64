#!/usr/bin/env python3
"""Self-test RTL synthesis constraint checker failure modes."""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_synth_constraints.py"
MANIFEST = ROOT / "fpga/constraints/lnp64_s0_smoke.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def run_checker(manifest: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_SYNTH_CONSTRAINTS_MANIFEST"] = str(manifest)
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


def main() -> None:
    base = json.loads(MANIFEST.read_text(encoding="utf-8"))
    valid = run_checker(MANIFEST)
    require(valid.returncode == 0, f"current synthesis constraints failed: {valid.stdout}")
    require("rtl synthesis constraints ok" in valid.stdout, "current synthesis constraints did not print success")

    with tempfile.TemporaryDirectory(prefix="lnp64-synth-constraints-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)

        missing_top_port = copy.deepcopy(base)
        missing_top_port["required_top_ports"].append("missing_manifest_port")
        missing_port_path = tmp / "missing_top_port.json"
        write_json(missing_port_path, missing_top_port)
        expect_failure(missing_port_path, "top port not constrained in manifest: missing_manifest_port")

        wrong_uart_byte = copy.deepcopy(base)
        wrong_uart_byte["fpga_uart_sim"]["expected_uart_hex"] = "54"
        wrong_uart_path = tmp / "wrong_uart_byte.json"
        write_json(wrong_uart_path, wrong_uart_byte)
        expect_failure(wrong_uart_path, "unexpected FPGA UART simulation byte")

        missing_board_env = copy.deepcopy(base)
        missing_board_env["board_live_validation"]["required_env"] = []
        missing_env_path = tmp / "missing_board_env.json"
        write_json(missing_env_path, missing_board_env)
        expect_failure(missing_env_path, "board validation must require UART device")

    print("rtl synthesis constraints checker self-test ok")


if __name__ == "__main__":
    main()

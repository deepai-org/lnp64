#!/usr/bin/env python3
"""Self-test FPGA timing/resource report checkers."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ICE40_CHECKER = ROOT / "scripts/check_ice40_report.py"
ICETIME_CHECKER = ROOT / "scripts/check_icetime_report.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    result.stdout = (result.stdout or "") + (result.stderr or "")
    return result


def expect_success(command: list[str], expected: str) -> None:
    result = run(command)
    require(result.returncode == 0, f"expected checker success: {result.stdout}")
    require(expected in result.stdout, f"checker success did not include {expected!r}: {result.stdout}")


def expect_failure(command: list[str], expected: str) -> None:
    result = run(command)
    require(result.returncode != 0, "expected checker failure")
    require(expected in result.stdout, f"checker failure did not include {expected!r}: {result.stdout}")


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="lnp64-fpga-report-checkers-") as raw_tmp:
        tmp = Path(raw_tmp)

        nextpnr_report = tmp / "nextpnr.json"
        valid_nextpnr = {
            "fmax": {"clk": {"achieved": 40.0, "constraint": 12.0}},
            "utilization": {
                "ICESTORM_LC": {"used": 10, "available": 7680},
                "SB_IO": {"used": 8, "available": 206},
            },
            "critical_paths": [{"from": "a", "to": "b"}],
        }
        write_json(nextpnr_report, valid_nextpnr)
        expect_success(
            [sys.executable, str(ICE40_CHECKER), "--report", str(nextpnr_report), "--min-frequency-mhz", "12"],
            "ice40 timing ok fmax=40.000MHz",
        )

        low_fmax = tmp / "nextpnr_low_fmax.json"
        low_fmax_data = dict(valid_nextpnr)
        low_fmax_data["fmax"] = {"clk": {"achieved": 10.0, "constraint": 12.0}}
        write_json(low_fmax, low_fmax_data)
        expect_failure(
            [sys.executable, str(ICE40_CHECKER), "--report", str(low_fmax), "--min-frequency-mhz", "12"],
            "clock clk achieved 10.000 MHz below required 12.000 MHz",
        )

        overused = tmp / "nextpnr_overused.json"
        overused_data = dict(valid_nextpnr)
        overused_data["utilization"] = {
            "ICESTORM_LC": {"used": 8000, "available": 7680},
            "SB_IO": {"used": 8, "available": 206},
        }
        write_json(overused, overused_data)
        expect_failure(
            [sys.executable, str(ICE40_CHECKER), "--report", str(overused), "--min-frequency-mhz", "12"],
            "ICESTORM_LC utilization exceeds availability",
        )

        icetime_summary = tmp / "icetime.summary"
        icetime_summary.write_text(
            "Timing estimate: 24.22 ns (41.27 MHz)\nclock constraint: PASSED\n",
            encoding="utf-8",
        )
        expect_success(
            [sys.executable, str(ICETIME_CHECKER), "--summary", str(icetime_summary), "--min-frequency-mhz", "12"],
            "icetime timing ok fmax=41.27MHz",
        )

        failed_constraint = tmp / "icetime_failed.summary"
        failed_constraint.write_text(
            "Timing estimate: 24.22 ns (41.27 MHz)\nclock constraint: FAILED\n",
            encoding="utf-8",
        )
        expect_failure(
            [sys.executable, str(ICETIME_CHECKER), "--summary", str(failed_constraint), "--min-frequency-mhz", "12"],
            "icetime report does not show a passed clock constraint",
        )

        missing_timing = tmp / "icetime_missing.summary"
        missing_timing.write_text("clock constraint: PASSED\n", encoding="utf-8")
        expect_failure(
            [sys.executable, str(ICETIME_CHECKER), "--summary", str(missing_timing), "--min-frequency-mhz", "12"],
            "icetime report missing timing estimate",
        )

    print("fpga report checkers self-test ok")


if __name__ == "__main__":
    main()

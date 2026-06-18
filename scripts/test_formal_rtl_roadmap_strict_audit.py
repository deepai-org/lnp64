#!/usr/bin/env python3
"""Self-test strict roadmap audit wiring with synthetic board evidence."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
AUDIT = ROOT / "scripts/check_formal_rtl_roadmap_audit.py"
CHECKLIST = ROOT / "formal_rtl_roadmap_completion_checklist.md"
REQUIRED_TOOLS = (
    "iceprog",
    "yosys",
    "nextpnr-ice40",
    "icepack",
    "icetime",
    "verilator",
    "python3",
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_strict(evidence: Path, extra_env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["LNP64_SKIP_STRICT_ROADMAP_AUDIT_SELFTEST"] = "1"
    if extra_env:
        env.update(extra_env)
    result = subprocess.run(
        [
            sys.executable,
            str(AUDIT),
            "--require-board-evidence",
            "--board-evidence",
            str(evidence),
        ],
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    result.stdout = (result.stdout or "") + (result.stderr or "")
    return result


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="lnp64-strict-roadmap-audit-") as raw_tmp:
        tmp = Path(raw_tmp)
        bitstream = tmp / "lnp64_s0_ice40.bin"
        uart_log = tmp / "uart.bin"
        preflight_log = tmp / "preflight.log"
        program_log = tmp / "program.log"
        evidence_path = tmp / "evidence.json"

        bitstream_bytes = b"synthetic strict roadmap audit bitstream\n"
        bitstream.write_bytes(bitstream_bytes)
        uart_log.write_bytes(bytes([0x53]))
        preflight_log.write_text("rtl board ice40 s0 preflight ok\n", encoding="utf-8")
        program_log.write_text("rtl board ice40 s0 program ok\n", encoding="utf-8")

        evidence = {
            "schema": "lnp64_board_ice40_s0_v1",
            "generated_at_utc": "2026-06-18T00:00:00Z",
            "target": "ice40-hx8k-ct256",
            "top": "lnp64_s0_fpga_top",
            "programmer": "iceprog",
            "bitstream": str(bitstream),
            "bitstream_sha256": sha256(bitstream_bytes),
            "uart_device": "/dev/ttyUSB0",
            "uart_baud": 115200,
            "expected_uart_hex": "53",
            "captured_uart_hex": "53",
            "uart_log": str(uart_log),
            "preflight_log": str(preflight_log),
            "program_log": str(program_log),
            "program_success_line": "rtl board ice40 s0 program ok",
            "success_line": "rtl board ice40 s0 live uart ok",
            "tool_versions": {
                tool: {
                    "path": f"/usr/bin/{tool}",
                    "version_probe": f"{tool} synthetic version",
                    "probe_status": 0,
                }
                for tool in REQUIRED_TOOLS
            },
        }
        write_json(evidence_path, evidence)

        strict = run_strict(evidence_path)
        require(strict.returncode == 0, f"strict audit rejected valid synthetic evidence: {strict.stdout}")
        require("board evidence ok" in strict.stdout, "strict audit did not run board evidence checker")
        require("formal RTL roadmap audit ok" in strict.stdout, "strict audit did not print success")

        missing = run_strict(tmp / "missing-evidence.json")
        require(missing.returncode != 0, "strict audit accepted missing evidence")
        require(
            "missing required live board evidence" in missing.stdout,
            f"strict missing-evidence failure was unclear: {missing.stdout}",
        )

        bad_checklist = tmp / "missing-track-b-row.md"
        bad_checklist.write_text(
            CHECKLIST.read_text(encoding="utf-8").replace(
                "| B6 gate/continuation block |",
                "| removed gate/continuation block |",
                1,
            ),
            encoding="utf-8",
        )
        missing_row = run_strict(
            evidence_path,
            {"LNP64_COMPLETION_CHECKLIST": str(bad_checklist)},
        )
        require(missing_row.returncode != 0, "strict audit accepted checklist missing a Track B row")
        require(
            "completion checklist omits Track B row" in missing_row.stdout,
            f"strict missing-checklist-row failure was unclear: {missing_row.stdout}",
        )

    print("formal RTL strict audit self-test ok")


if __name__ == "__main__":
    main()

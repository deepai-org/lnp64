#!/usr/bin/env python3
"""Self-test the live board evidence validator with synthetic artifacts."""

from __future__ import annotations

import copy
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_board_evidence.py"
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


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_checker(evidence: Path) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        [sys.executable, str(CHECKER), str(evidence)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    result.stdout = (result.stdout or "") + (result.stderr or "")
    return result


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def expect_failure(evidence: Path, expected: str) -> None:
    result = run_checker(evidence)
    require(result.returncode != 0, f"expected checker failure for {evidence}")
    require(expected in result.stdout, f"checker failure did not include {expected!r}: {result.stdout}")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="lnp64-board-evidence-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)
        bitstream = tmp / "lnp64_s0_ice40.bin"
        uart_log = tmp / "uart.bin"
        preflight_log = tmp / "preflight.log"
        program_log = tmp / "program.log"
        evidence_path = tmp / "evidence.json"

        bitstream_bytes = b"synthetic lnp64 s0 ice40 bitstream\n"
        uart_bytes = bytes([0x00, 0x53, 0x0A])
        bitstream.write_bytes(bitstream_bytes)
        uart_log.write_bytes(uart_bytes)
        preflight_log.write_text("probe output\nrtl board ice40 s0 preflight ok\n", encoding="utf-8")
        program_log.write_text("iceprog output\nrtl board ice40 s0 program ok\n", encoding="utf-8")

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
            "captured_uart_hex": "00 53 0a",
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

        result = run_checker(evidence_path)
        require(result.returncode == 0, f"valid synthetic evidence failed: {result.stdout}")
        require("board evidence ok" in result.stdout, "valid synthetic evidence did not print success")

        bad_hash = copy.deepcopy(evidence)
        bad_hash["bitstream_sha256"] = "0" * 64
        bad_hash_path = tmp / "bad_hash.json"
        write_json(bad_hash_path, bad_hash)
        expect_failure(bad_hash_path, "bitstream SHA-256 does not match evidence")

        bad_uart = copy.deepcopy(evidence)
        bad_uart["captured_uart_hex"] = "00 53"
        bad_uart_path = tmp / "bad_uart.json"
        write_json(bad_uart_path, bad_uart)
        expect_failure(bad_uart_path, "captured UART bytes do not match UART log contents")

        bad_expected = copy.deepcopy(evidence)
        bad_expected["expected_uart_hex"] = "54"
        bad_expected_path = tmp / "bad_expected.json"
        write_json(bad_expected_path, bad_expected)
        expect_failure(bad_expected_path, "unexpected board UART expected byte")

        bad_preflight = copy.deepcopy(evidence)
        bad_preflight_log = tmp / "bad_preflight.log"
        bad_preflight_log.write_text("probe output without success marker\n", encoding="utf-8")
        bad_preflight["preflight_log"] = str(bad_preflight_log)
        bad_preflight_path = tmp / "bad_preflight.json"
        write_json(bad_preflight_path, bad_preflight)
        expect_failure(bad_preflight_path, "preflight log does not include success line")

        bad_timestamp = copy.deepcopy(evidence)
        bad_timestamp["generated_at_utc"] = "not-a-timestamp"
        bad_timestamp_path = tmp / "bad_timestamp.json"
        write_json(bad_timestamp_path, bad_timestamp)
        expect_failure(bad_timestamp_path, "missing or invalid board evidence UTC timestamp")

        bad_device = copy.deepcopy(evidence)
        bad_device["uart_device"] = "/dev/null"
        bad_device_path = tmp / "bad_device.json"
        write_json(bad_device_path, bad_device)
        expect_failure(bad_device_path, "UART device does not look like a serial TTY")

    build_dir = ROOT / "build"
    build_dir.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix="lnp64-board-evidence-workpath-",
        dir=build_dir,
    ) as raw_work_tmp:
        work_tmp = Path(raw_work_tmp)
        bitstream = work_tmp / "lnp64_s0_ice40.bin"
        uart_log = work_tmp / "uart.bin"
        preflight_log = work_tmp / "preflight.log"
        program_log = work_tmp / "program.log"

        bitstream_bytes = b"synthetic /work mounted bitstream\n"
        bitstream.write_bytes(bitstream_bytes)
        uart_log.write_bytes(bytes([0x53]))
        preflight_log.write_text("rtl board ice40 s0 preflight ok\n", encoding="utf-8")
        program_log.write_text("rtl board ice40 s0 program ok\n", encoding="utf-8")

        def work_path(path: Path) -> str:
            return "/work/" + path.relative_to(ROOT).as_posix()

        evidence = {
            "schema": "lnp64_board_ice40_s0_v1",
            "generated_at_utc": "2026-06-18T00:00:00Z",
            "target": "ice40-hx8k-ct256",
            "top": "lnp64_s0_fpga_top",
            "programmer": "iceprog",
            "bitstream": work_path(bitstream),
            "bitstream_sha256": sha256(bitstream_bytes),
            "uart_device": "/dev/ttyUSB0",
            "uart_baud": 115200,
            "expected_uart_hex": "53",
            "captured_uart_hex": "53",
            "uart_log": work_path(uart_log),
            "preflight_log": work_path(preflight_log),
            "program_log": work_path(program_log),
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
        evidence_path = work_tmp / "work_path_evidence.json"
        write_json(evidence_path, evidence)

        result = run_checker(evidence_path)
        require(result.returncode == 0, f"/work synthetic evidence failed: {result.stdout}")
        require("board evidence ok" in result.stdout, "/work synthetic evidence did not print success")

    print("board evidence checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Validate a live iCE40 S0 board bring-up evidence file."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
BYTE_HEX_RE = re.compile(r"^[0-9a-f]{2}$")
UTC_TIMESTAMP_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
SERIAL_TTY_RE = re.compile(r"^/dev/tty(USB|ACM|AMA|S)\d+$")
REQUIRED_TOOLS = (
    "iceprog",
    "yosys",
    "nextpnr-ice40",
    "icepack",
    "icetime",
    "verilator",
    "python3",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", help="JSON evidence file from scripts/run_rtl_board_ice40_s0.sh")
    return parser.parse_args()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def resolve_artifact(raw: object, evidence_path: Path) -> Path:
    require(isinstance(raw, str) and raw, "missing artifact path")
    path = Path(raw)
    if path.exists():
        return path
    if path.is_absolute() and len(path.parts) > 2 and path.parts[1] == "work":
        repo_path = Path.cwd() / Path(*path.parts[2:])
        if repo_path.exists():
            return repo_path
    sibling_path = evidence_path.parent / path.name
    if sibling_path.exists():
        return sibling_path
    return path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def bytes_to_hex(data: bytes) -> str:
    return " ".join(f"{byte:02x}" for byte in data)


def main() -> None:
    args = parse_args()
    path = Path(args.evidence)
    require(path.exists(), f"missing board evidence file {path}")
    evidence = json.loads(path.read_text(encoding="utf-8"))

    require(evidence.get("schema") == "lnp64_board_ice40_s0_v1", "unexpected board evidence schema")
    generated_at = evidence.get("generated_at_utc")
    require(
        isinstance(generated_at, str) and UTC_TIMESTAMP_RE.match(generated_at) is not None,
        "missing or invalid board evidence UTC timestamp",
    )
    require(evidence.get("target") == "ice40-hx8k-ct256", "unexpected board target")
    require(evidence.get("top") == "lnp64_s0_fpga_top", "unexpected board top")
    require(evidence.get("programmer") == "iceprog", "unexpected board programmer")
    require(evidence.get("program_success_line") == "rtl board ice40 s0 program ok", "missing board program success line")
    require(evidence.get("uart_baud") == 115200, "unexpected board UART baud")
    expected_uart_hex = evidence.get("expected_uart_hex")
    require(isinstance(expected_uart_hex, str), "missing board UART expected byte")
    expected_uart_hex = expected_uart_hex.lower()
    require(BYTE_HEX_RE.match(expected_uart_hex) is not None, "invalid board UART expected byte")
    require(expected_uart_hex == "53", "unexpected board UART expected byte")
    require(evidence.get("success_line") == "rtl board ice40 s0 live uart ok", "missing board success line")

    tool_versions = evidence.get("tool_versions")
    require(isinstance(tool_versions, dict), "missing tool_versions")
    for tool in REQUIRED_TOOLS:
        entry = tool_versions.get(tool)
        require(isinstance(entry, dict), f"missing tool_versions entry for {tool}")
        require(isinstance(entry.get("path"), str) and entry["path"], f"missing tool path for {tool}")
        require(isinstance(entry.get("version_probe"), str) and entry["version_probe"], f"missing tool version probe for {tool}")
        require(isinstance(entry.get("probe_status"), int), f"missing tool probe status for {tool}")

    bitstream_hash = evidence.get("bitstream_sha256")
    require(isinstance(bitstream_hash, str) and SHA256_RE.match(bitstream_hash), "invalid bitstream SHA-256")
    bitstream = resolve_artifact(evidence.get("bitstream"), path)
    require(bitstream.exists(), f"missing bitstream artifact {bitstream}")
    require(bitstream.stat().st_size > 0, f"empty bitstream artifact {bitstream}")
    require(sha256_file(bitstream) == bitstream_hash, "bitstream SHA-256 does not match evidence")

    uart_device = evidence.get("uart_device")
    require(isinstance(uart_device, str) and uart_device, "missing UART device")
    require(SERIAL_TTY_RE.match(uart_device) is not None, f"UART device does not look like a serial TTY: {uart_device}")

    preflight_log = resolve_artifact(evidence.get("preflight_log"), path)
    require(preflight_log.exists(), f"missing board artifact {preflight_log}")
    require(preflight_log.stat().st_size > 0, f"empty board artifact {preflight_log}")
    require(
        "rtl board ice40 s0 preflight ok" in preflight_log.read_text(encoding="utf-8", errors="replace"),
        "preflight log does not include success line",
    )

    for key in ("uart_log", "program_log"):
        artifact = resolve_artifact(evidence.get(key), path)
        require(artifact.exists(), f"missing board artifact {artifact}")
        require(artifact.stat().st_size > 0, f"empty board artifact {artifact}")

    program_log = resolve_artifact(evidence.get("program_log"), path)
    require(
        "rtl board ice40 s0 program ok" in program_log.read_text(encoding="utf-8", errors="replace"),
        "program log does not include success line",
    )

    captured_uart_hex = evidence.get("captured_uart_hex")
    require(isinstance(captured_uart_hex, str), "missing captured UART bytes")
    captured = captured_uart_hex.lower().split()
    require(
        expected_uart_hex in captured,
        f"captured UART bytes do not include expected byte 0x{expected_uart_hex}",
    )

    uart_log = resolve_artifact(evidence.get("uart_log"), path)
    uart_log_hex = bytes_to_hex(uart_log.read_bytes())
    require(
        expected_uart_hex in uart_log_hex.split(),
        f"UART log bytes do not include expected byte 0x{expected_uart_hex}",
    )
    require(
        uart_log_hex == captured_uart_hex.lower(),
        "captured UART bytes do not match UART log contents",
    )

    print("board evidence ok")


if __name__ == "__main__":
    main()

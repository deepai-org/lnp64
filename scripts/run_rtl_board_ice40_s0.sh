#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v iceprog >/dev/null 2>&1; then
  printf '%s\n' "iceprog is required for the RTL board iCE40 S0 gate" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' "python3 is required for the RTL board iCE40 S0 gate" >&2
  exit 1
fi

uart_baud="${LNP64_BOARD_UART_BAUD:-115200}"
if [ "$uart_baud" != "115200" ]; then
  printf 'LNP64_BOARD_UART_BAUD must be 115200 for the S0 board evidence schema: %s\n' "$uart_baud" >&2
  exit 1
fi

preflight_log="${LNP64_BOARD_PREFLIGHT_LOG:-/tmp/lnp64-board-ice40-s0-preflight.log}"
export LNP64_BOARD_PREFLIGHT_LOG="$preflight_log"
scripts/run_rtl_board_preflight.sh
uart_device="$(readlink -f "${LNP64_BOARD_UART_DEVICE}")"

tmpdir="$(mktemp -d /tmp/lnp64-board-ice40-s0-XXXXXX)"
uart_pid=""
cleanup() {
  if [ -n "$uart_pid" ]; then
    kill "$uart_pid" 2>/dev/null || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

evidence_out="${LNP64_BOARD_EVIDENCE_OUT:-/tmp/lnp64-board-ice40-s0-evidence.json}"
bin_in="${LNP64_ICE40_BIN_IN:-}"
bin_out="${LNP64_ICE40_BIN_OUT:-$(dirname "$evidence_out")/lnp64_s0_ice40.bin}"
uart_log="${LNP64_BOARD_UART_LOG:-$tmpdir/uart.bin}"
program_log="${LNP64_BOARD_PROGRAM_LOG:-/tmp/lnp64-board-ice40-s0-iceprog.log}"
uart_timeout="${LNP64_BOARD_UART_TIMEOUT:-10}"
expected_byte="${LNP64_BOARD_UART_EXPECT_HEX:-53}"
expected_byte="$(
  python3 - "$expected_byte" <<'PY'
from __future__ import annotations

import sys

raw = sys.argv[1]
try:
    value = int(raw, 16)
except ValueError:
    raise SystemExit(f"invalid LNP64_BOARD_UART_EXPECT_HEX: {raw}")
if value < 0 or value > 0xFF:
    raise SystemExit(f"LNP64_BOARD_UART_EXPECT_HEX is not a byte: {raw}")
print(f"{value:02x}")
PY
)"
program_success_line="rtl board ice40 s0 program ok"
success_line="rtl board ice40 s0 live uart ok"
iceprog_args=()

if [ -n "${LNP64_ICEPROG_DEVICE:-}" ]; then
  iceprog_args+=( -d "$LNP64_ICEPROG_DEVICE" )
fi
if [ -n "${LNP64_ICEPROG_INTERFACE:-}" ]; then
  iceprog_args+=( -I "$LNP64_ICEPROG_INTERFACE" )
fi
if [ "${LNP64_ICEPROG_SLOW:-0}" = "1" ]; then
  iceprog_args+=( -s )
fi

mkdir -p "$(dirname "$uart_log")" "$(dirname "$program_log")" "$(dirname "$preflight_log")" "$(dirname "$evidence_out")"

if [ -n "$bin_in" ]; then
  if [ ! -s "$bin_in" ]; then
    printf 'Configured bitstream is missing or empty: %s\n' "$bin_in" >&2
    exit 1
  fi
  bitstream="$bin_in"
else
  LNP64_ICE40_BIN_OUT="$bin_out" bash scripts/run_rtl_fpga_ice40_s0.sh
  bitstream="$bin_out"
fi

python3 scripts/check_uart_byte.py \
  --device "$uart_device" \
  --baud "$uart_baud" \
  --expect-hex "$expected_byte" \
  --timeout "$uart_timeout" \
  --output "$uart_log" &
uart_pid="$!"

sleep "${LNP64_BOARD_CAPTURE_SETTLE_SECONDS:-0.2}"
iceprog "${iceprog_args[@]}" "$bitstream" 2>&1 | tee "$program_log"
printf '%s\n' "$program_success_line" | tee -a "$program_log"

wait "$uart_pid"
uart_pid=""

bitstream_sha256="$(sha256sum "$bitstream" | awk '{print $1}')"
captured_uart_hex="$(
  python3 - "$uart_log" <<'PY'
from pathlib import Path
import sys

data = Path(sys.argv[1]).read_bytes()
print(" ".join(f"{byte:02x}" for byte in data))
PY
)"

python3 - \
  "$evidence_out" \
  "$bitstream" \
  "$bitstream_sha256" \
  "$uart_device" \
  "$uart_baud" \
  "$expected_byte" \
  "$uart_log" \
  "$captured_uart_hex" \
  "$preflight_log" \
  "$program_log" \
  "$program_success_line" \
  "$success_line" <<'PY'
from __future__ import annotations

import json
import shutil
import subprocess
import time
from pathlib import Path
import sys

(
    evidence_out,
    bitstream,
    bitstream_sha256,
    uart_device,
    uart_baud,
    expected_byte,
    uart_log,
    captured_uart_hex,
    preflight_log,
    program_log,
    program_success_line,
    success_line,
) = sys.argv[1:]

TOOL_PROBES = {
    "iceprog": ["iceprog", "--help"],
    "yosys": ["yosys", "-V"],
    "nextpnr-ice40": ["nextpnr-ice40", "--version"],
    "icepack": ["icepack", "-h"],
    "icetime": ["icetime", "-h"],
    "verilator": ["verilator", "--version"],
    "python3": ["python3", "--version"],
}


def probe_tool(name: str, command: list[str]) -> dict[str, str | int | None]:
    path = shutil.which(name)
    result = {"path": path, "version_probe": "", "probe_status": None}
    if path is None:
        return result
    probe = subprocess.run(command, text=True, capture_output=True, check=False)
    output_text = (probe.stdout or "") + (probe.stderr or "")
    output = " ".join(line.strip() for line in output_text.splitlines() if line.strip())
    result["version_probe"] = output[:240]
    result["probe_status"] = probe.returncode
    return result


evidence = {
    "schema": "lnp64_board_ice40_s0_v1",
    "generated_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "target": "ice40-hx8k-ct256",
    "top": "lnp64_s0_fpga_top",
    "programmer": "iceprog",
    "bitstream": str(Path(bitstream)),
    "bitstream_sha256": bitstream_sha256,
    "uart_device": uart_device,
    "uart_baud": int(uart_baud),
    "expected_uart_hex": expected_byte.lower(),
    "captured_uart_hex": captured_uart_hex,
    "uart_log": str(Path(uart_log)),
    "preflight_log": str(Path(preflight_log)),
    "program_log": str(Path(program_log)),
    "program_success_line": program_success_line,
    "success_line": success_line,
    "tool_versions": {
        name: probe_tool(name, command) for name, command in TOOL_PROBES.items()
    },
}
Path(evidence_out).write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

scripts/check_board_evidence.py "$evidence_out"

printf 'board evidence: %s\n' "$evidence_out"
printf '%s\n' "$success_line"

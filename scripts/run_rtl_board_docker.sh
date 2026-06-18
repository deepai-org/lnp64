#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_RTL_BOARD_IMAGE:-lnp64-rtl-board}"
uart_device="${LNP64_BOARD_UART_DEVICE:-}"

if [ -z "$uart_device" ]; then
  printf '%s\n' "LNP64_BOARD_UART_DEVICE must be set before running the board Docker gate" >&2
  exit 1
fi

if [ ! -e "$uart_device" ]; then
  printf 'Board UART device does not exist: %s\n' "$uart_device" >&2
  exit 1
fi

resolved_uart="$(readlink -f "$uart_device")"
if [ ! -c "$resolved_uart" ]; then
  printf 'Board UART device is not a character device: %s -> %s\n' "$uart_device" "$resolved_uart" >&2
  exit 1
fi

case "$resolved_uart" in
  /dev/ttyUSB*|/dev/ttyACM*|/dev/ttyAMA*|/dev/ttyS*)
    ;;
  *)
    printf 'Board UART device does not look like a serial TTY: %s -> %s\n' "$uart_device" "$resolved_uart" >&2
    exit 1
    ;;
esac

mkdir -p build

docker build \
  -f Dockerfile.rtl-board \
  -t "$image" \
  .

docker run --rm \
  --privileged \
  --device "$resolved_uart:$resolved_uart" \
  -e LNP64_BOARD_UART_DEVICE="$resolved_uart" \
  -e LNP64_BOARD_UART_BAUD="${LNP64_BOARD_UART_BAUD:-115200}" \
  -e LNP64_BOARD_UART_TIMEOUT="${LNP64_BOARD_UART_TIMEOUT:-10}" \
  -e LNP64_BOARD_UART_EXPECT_HEX="${LNP64_BOARD_UART_EXPECT_HEX:-53}" \
  -e LNP64_BOARD_EVIDENCE_OUT="${LNP64_BOARD_EVIDENCE_OUT:-/work/build/lnp64-board-ice40-s0-evidence.json}" \
  -e LNP64_BOARD_UART_LOG="${LNP64_BOARD_UART_LOG:-/work/build/lnp64-board-ice40-s0-uart.bin}" \
  -e LNP64_BOARD_PROGRAM_LOG="${LNP64_BOARD_PROGRAM_LOG:-/work/build/lnp64-board-ice40-s0-iceprog.log}" \
  -e LNP64_BOARD_PREFLIGHT_LOG="${LNP64_BOARD_PREFLIGHT_LOG:-/work/build/lnp64-board-ice40-s0-preflight.log}" \
  -e LNP64_BOARD_CAPTURE_SETTLE_SECONDS="${LNP64_BOARD_CAPTURE_SETTLE_SECONDS:-0.2}" \
  -e LNP64_BOARD_SKIP_PROGRAMMER_PROBE="${LNP64_BOARD_SKIP_PROGRAMMER_PROBE:-0}" \
  -e LNP64_ICE40_BIN_IN="${LNP64_ICE40_BIN_IN:-}" \
  -e LNP64_ICE40_BIN_OUT="${LNP64_ICE40_BIN_OUT:-/work/build/lnp64_s0_ice40.bin}" \
  -e LNP64_ICEPROG_DEVICE="${LNP64_ICEPROG_DEVICE:-}" \
  -e LNP64_ICEPROG_INTERFACE="${LNP64_ICEPROG_INTERFACE:-}" \
  -e LNP64_ICEPROG_SLOW="${LNP64_ICEPROG_SLOW:-0}" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_rtl_board_ice40_s0.sh

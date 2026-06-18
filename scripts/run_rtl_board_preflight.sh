#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v iceprog >/dev/null 2>&1; then
  printf '%s\n' "iceprog is required for the RTL board iCE40 gate" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' "python3 is required for the RTL board iCE40 gate" >&2
  exit 1
fi

if ! python3 - <<'PY' >/dev/null 2>&1
import serial
PY
then
  printf '%s\n' "python3-serial is required for the RTL board iCE40 gate" >&2
  exit 1
fi

uart_device="${LNP64_BOARD_UART_DEVICE:-}"
if [ -z "$uart_device" ]; then
  printf '%s\n' "LNP64_BOARD_UART_DEVICE must name the board UART serial device for live validation" >&2
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

if command -v lsusb >/dev/null 2>&1; then
  printf '%s\n' "USB devices visible to board gate:"
  lsusb || true
fi

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

probe_log="${LNP64_BOARD_PREFLIGHT_LOG:-/tmp/lnp64-board-ice40-s0-preflight.log}"
mkdir -p "$(dirname "$probe_log")"

if [ "${LNP64_BOARD_SKIP_PROGRAMMER_PROBE:-0}" != "1" ]; then
  if ! iceprog "${iceprog_args[@]}" -t >"$probe_log" 2>&1; then
    cat "$probe_log" >&2 || true
    printf '%s\n' "iceprog preflight probe failed; no compatible iCE40 programmer was confirmed" >&2
    exit 1
  fi
else
  printf '%s\n' "iceprog preflight probe skipped by LNP64_BOARD_SKIP_PROGRAMMER_PROBE=1" >"$probe_log"
fi

{
  cat "$probe_log"
  printf '%s\n' "rtl board ice40 s0 preflight ok"
} >"${probe_log}.tmp"
mv "${probe_log}.tmp" "$probe_log"

if [ -s "$probe_log" ]; then
  cat "$probe_log"
fi

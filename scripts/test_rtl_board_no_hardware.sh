#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tmpdir="$(mktemp -d /tmp/lnp64-board-no-hardware-test-XXXXXX)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

if env -u LNP64_BOARD_UART_DEVICE bash scripts/run_rtl_board_docker.sh >"$tmpdir/no-env.out" 2>&1; then
  printf '%s\n' "board Docker gate unexpectedly passed without LNP64_BOARD_UART_DEVICE" >&2
  exit 1
fi
grep -q "LNP64_BOARD_UART_DEVICE must be set" "$tmpdir/no-env.out"

if LNP64_BOARD_UART_DEVICE=/dev/null bash scripts/run_rtl_board_docker.sh >"$tmpdir/not-tty.out" 2>&1; then
  printf '%s\n' "board Docker gate unexpectedly passed with /dev/null as UART" >&2
  exit 1
fi
grep -q "Board UART device does not look like a serial TTY" "$tmpdir/not-tty.out"

mkdir -p "$tmpdir/bin"
cat > "$tmpdir/bin/iceprog" <<'SH'
#!/usr/bin/env bash
exit 0
SH
chmod +x "$tmpdir/bin/iceprog"

cat > "$tmpdir/bin/python3" <<SH
#!/usr/bin/env bash
exec "$(command -v python3)" "\$@"
SH
chmod +x "$tmpdir/bin/python3"

cat > "$tmpdir/preflight.sh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "preflight should not run when baud is invalid" >&2
exit 1
SH
chmod +x "$tmpdir/preflight.sh"

if PATH="$tmpdir/bin:$PATH" \
    LNP64_BOARD_UART_DEVICE=/dev/ttyUSB0 \
    LNP64_BOARD_UART_BAUD=9600 \
    bash scripts/run_rtl_board_ice40_s0.sh >"$tmpdir/bad-baud.out" 2>&1; then
  printf '%s\n' "board gate unexpectedly passed with invalid UART baud" >&2
  exit 1
fi
grep -q "LNP64_BOARD_UART_BAUD must be 115200" "$tmpdir/bad-baud.out"

printf '%s\n' "rtl board no-hardware self-test ok"

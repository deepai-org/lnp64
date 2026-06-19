#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --quiet --)
fi

"${lnp64[@]}" cc --toy-bootstrap \
  third_party/zlib/smoke.c \
  third_party/zlib/adler32.c \
  third_party/zlib/crc32.c \
  third_party/zlib/zutil.c \
  -o /tmp/zlib_smoke.s
out=$("${lnp64[@]}" run /tmp/zlib_smoke.s)
test "$out" = "zlib checksum ok"
printf '%s\n' "$out"

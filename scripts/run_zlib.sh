#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --quiet --)

"${lnp64[@]}" cc \
  third_party/zlib/smoke.c \
  third_party/zlib/adler32.c \
  third_party/zlib/zutil.c \
  -o /tmp/zlib_smoke.s
out=$("${lnp64[@]}" run /tmp/zlib_smoke.s)
test "$out" = "zlib adler32 ok"
printf '%s\n' "$out"

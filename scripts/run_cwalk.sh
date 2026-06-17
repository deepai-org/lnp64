#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --quiet --)

"${lnp64[@]}" cc \
  third_party/cwalk/smoke.c \
  third_party/cwalk/include/cwalk.h \
  third_party/cwalk/src/cwalk.c \
  -o /tmp/cwalk_smoke.s
out=$("${lnp64[@]}" run /tmp/cwalk_smoke.s)
test "$out" = "cwalk ok"
printf '%s\n' "$out"

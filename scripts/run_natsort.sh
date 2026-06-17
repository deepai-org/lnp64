#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --quiet --)

"${lnp64[@]}" cc \
  third_party/natsort/smoke.c \
  third_party/natsort/strnatcmp.c \
  -o /tmp/natsort_smoke.s
out=$("${lnp64[@]}" run /tmp/natsort_smoke.s)
test "$out" = "natsort ok"
printf '%s\n' "$out"

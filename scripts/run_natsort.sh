#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --quiet --)
fi

"${lnp64[@]}" cc --toy-bootstrap \
  third_party/natsort/smoke.c \
  third_party/natsort/strnatcmp.c \
  -o /tmp/natsort_smoke.s
out=$("${lnp64[@]}" run /tmp/natsort_smoke.s)
test "$out" = "natsort ok"
printf '%s\n' "$out"

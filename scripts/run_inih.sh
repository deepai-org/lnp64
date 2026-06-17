#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --release --quiet --)

"${lnp64[@]}" cc third_party/inih/smoke.c third_party/inih/ini.c -o /tmp/inih_smoke.s
out=$("${lnp64[@]}" run /tmp/inih_smoke.s -- inih_smoke)
test "$out" = "inih ok"
echo "$out"

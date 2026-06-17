#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --release --quiet --)
asm=/tmp/netbsd_personality_smoke.s
out=/tmp/netbsd_personality_smoke.out

"${lnp64[@]}" cc demos/netbsd_personality_smoke.c -o "$asm"
rm -f "$out"
"${lnp64[@]}" run "$asm" > "$out"
cat "$out"

grep -q "netbsd personality init" "$out"
grep -q "netbsd personality shell" "$out"
grep -q "netbsd personality smoke ok" "$out"

printf '%s\n' "netbsd personality smoke gate ok"

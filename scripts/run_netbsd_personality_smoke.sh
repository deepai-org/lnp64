#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi
asm=/tmp/netbsd_personality_smoke.s
out=/tmp/netbsd_personality_smoke.out

"${lnp64[@]}" cc --toy-bootstrap demos/netbsd_personality_smoke.c -o "$asm"

required_native=(
  OPEN_AT
  PULL_DYN
  PUSH_DYN
  FORK
  EXEC
  SPAWN
  FUTEX_WAIT
  FUTEX_WAKE
  OBJECT_CTL
  MMAP
  MPROTECT
  MUNMAP
  POLL_FD_DYN
  AWAIT_DYN
  SIGACTION
  GET_PCR
  "SET_PCR r"
  KILL
  ALARM
  SLEEP
  CAP_DUP
  CAP_SEND
  CAP_RECV
  DOMAIN_CTL
  GATE_CALL
  GATE_RETURN
)

for token in "${required_native[@]}"; do
  grep -q "$token" "$asm"
done

rm -f "$out"
"${lnp64[@]}" run "$asm" > "$out"
cat "$out"

grep -q "netbsd personality init" "$out"
grep -q "netbsd personality shell" "$out"
grep -q "netbsd personality smoke ok" "$out"

printf '%s\n' "netbsd personality smoke gate ok"

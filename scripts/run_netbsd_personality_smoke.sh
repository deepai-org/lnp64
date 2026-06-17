#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --release --quiet --)
asm=/tmp/netbsd_personality_smoke.s
out=/tmp/netbsd_personality_smoke.out

"${lnp64[@]}" cc demos/netbsd_personality_smoke.c -o "$asm"

required_native=(
  OPEN_FD
  READ_FD_DYN
  WRITE_FD_DYN
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
  "SET_PCR SIGMASK"
  KILL
  ALARM
  SLEEP
  CAP_DUP
  CAP_SEND
  CAP_RECV
  DOMAIN_CTL
  CALL_CAP
  RET_CAP
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

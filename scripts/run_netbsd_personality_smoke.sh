#!/usr/bin/env bash
set -euo pipefail

mode="llvm"
usage() {
  cat <<'USAGE'
usage: scripts/run_netbsd_personality_smoke.sh [--backend llvm|toy] [--legacy-toy]

The default llvm backend runs the Clang/lld/run-elf NetBSD personality package
gate. --legacy-toy preserves the old single-file smoke that compiles C with
lnp64 cc --toy-bootstrap.
USAGE
}

while (($#)); do
  case "$1" in
    --backend)
      mode="${2:-}"
      if [[ -z "$mode" ]]; then
        printf '%s\n' "missing value for --backend" >&2
        usage >&2
        exit 2
      fi
      shift 2
      ;;
    --legacy-toy)
      mode="toy"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$mode" in
  llvm)
    printf '%s\n' "== real LLVM LNP64 package gate: netbsd =="
    LNP64_LLVM_PACKAGE_FILTER=netbsd bash scripts/run_real_llvm_package_gate.sh
    printf '%s\n' "netbsd personality smoke gate ok"
    exit 0
    ;;
  toy)
    ;;
  *)
    printf 'unknown backend: %s\n' "$mode" >&2
    usage >&2
    exit 2
    ;;
esac

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

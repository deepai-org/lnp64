#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

backend="llvm"
loader="exec-plan"

usage() {
  cat <<'USAGE'
usage: scripts/run_libc_test.sh [--backend llvm] [--loader exec-plan]

The default llvm/exec-plan backend dispatches to the real Clang/lld/run-elf
gate.
USAGE
}

while (($#)); do
  case "$1" in
    --backend)
      backend="${2:-}"
      if [[ -z "$backend" ]]; then
        printf '%s\n' "missing value for --backend" >&2
        usage >&2
        exit 2
      fi
      if [[ "$backend" != "llvm" ]]; then
        printf 'unknown backend: %s\n' "$backend" >&2
        usage >&2
        exit 2
      fi
      shift 2
      ;;
    --loader)
      loader="${2:-}"
      if [[ -z "$loader" ]]; then
        printf '%s\n' "missing value for --loader" >&2
        usage >&2
        exit 2
      fi
      if [[ "$loader" != "exec-plan" ]]; then
        printf '%s\n' "llvm backend requires --loader exec-plan" >&2
        exit 2
      fi
      shift 2
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

exec bash scripts/run_real_llvm_lnp64_docker.sh

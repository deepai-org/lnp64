#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/run_netbsd_personality_smoke.sh [--backend llvm]

The default llvm backend runs the Clang/lld/run-elf NetBSD personality package
gate.
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
      if [[ "$mode" != "llvm" ]]; then
        printf 'unknown backend: %s\n' "$mode" >&2
        usage >&2
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

printf '%s\n' "== real LLVM LNP64 package gate: netbsd =="
LNP64_LLVM_PACKAGE_FILTER=netbsd bash scripts/run_real_llvm_package_gate.sh
printf '%s\n' "netbsd personality smoke gate ok"

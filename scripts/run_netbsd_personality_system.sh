#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/run_netbsd_personality_system.sh [--backend llvm]

Runs the Clang/lld/run-elf NetBSD personality probes through the real LLVM
package gate, then checks the canonical native lowering contract and stale-FDR
assembly smoke that protect the system-level personality boundary.
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

cargo test --quiet netbsd_system_gate_canonical_native_primitives_cover_runner_requirements

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi
"${lnp64[@]}" run demos/stale_fd_token.s > "${TMPDIR:-/tmp}/lnp64-netbsd-stale.out"
grep -q "stale fd token ok" "${TMPDIR:-/tmp}/lnp64-netbsd-stale.out"

printf '%s\n' "netbsd personality system gate ok"

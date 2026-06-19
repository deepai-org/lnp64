#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

manifest="toolchain/lnp64_llvm_gates.manifest"
build_dir="${LNP64_LLVM_BUILD_DIR:-target/lnp64-llvm-bootstrap}"
mode="dry-run"

usage() {
  cat <<'USAGE'
usage: scripts/run_llvm_bootstrap_gates.sh [--dry-run|--run]

Dry-run prints the Clang/lld/loader gates from
toolchain/lnp64_llvm_gates.manifest. Running executes tested gates by default
and skips planned gates unless LNP64_RUN_PLANNED_LLVM_GATES=1 is set.
USAGE
}

while (($#)); do
  case "$1" in
    --dry-run)
      mode="dry-run"
      ;;
    --run)
      mode="run"
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
  shift
done

mkdir -p "$build_dir"

while IFS='|' read -r gate command requires status; do
  if [[ -z "${gate:-}" || "$gate" == \#* ]]; then
    continue
  fi
  if [[ -z "${command:-}" || -z "${requires:-}" || -z "${status:-}" ]]; then
    printf 'malformed LLVM gate row: %s|%s|%s|%s\n' "$gate" "$command" "$requires" "$status" >&2
    exit 1
  fi
  command="${command//\{build\}/$build_dir}"
  printf '[%s] %s\n' "$status" "$gate"
  printf '  requires: %s\n' "$requires"
  printf '  command: %s\n' "$command"
  if [[ "$mode" == "run" ]]; then
    if [[ "$status" == "planned" ]]; then
      if [[ "${LNP64_RUN_PLANNED_LLVM_GATES:-0}" != "1" ]]; then
        printf '  note: skipping planned gate without LNP64_RUN_PLANNED_LLVM_GATES=1\n'
        continue
      fi
      printf '  note: executing planned gate by explicit opt-in\n'
    fi
    eval "$command"
  fi
done < "$manifest"

if [[ "$mode" == "dry-run" ]]; then
  printf '%s\n' "llvm bootstrap gates dry-run only"
else
  printf '%s\n' "llvm bootstrap gates completed"
fi

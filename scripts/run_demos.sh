#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/run_demos.sh

Runs checked legacy-assembler smoke demos only. Real Clang/lld C demo coverage
lives in scripts/run_real_llvm_lnp64_docker.sh.
USAGE
}

while (($#)); do
  case "$1" in
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

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi

echo "== real clang/lld C demos =="
echo "Run LNP64_LLVM_DOCKER_SKIP_BUILD=1 bash scripts/run_real_llvm_lnp64_docker.sh"
echo "== legacy assembler smoke demos only =="

for src in demos/*.s; do
  echo "== $src =="
  "${lnp64[@]}" run "$src"
done

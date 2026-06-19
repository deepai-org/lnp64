#!/usr/bin/env bash
set -euo pipefail

include_legacy_toy=0
usage() {
  cat <<'USAGE'
usage: scripts/run_demos.sh [--legacy-toy]

Runs checked assembly demos by default. --legacy-toy also runs the remaining
toy-bootstrap C demo smoke for compatibility-personality coverage.
USAGE
}

while (($#)); do
  case "$1" in
    --legacy-toy)
      include_legacy_toy=1
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

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi

echo "== real clang/lld C demos =="
echo "Run LNP64_LLVM_DOCKER_SKIP_BUILD=1 bash scripts/run_real_llvm_lnp64_docker.sh"

legacy_toy_c=(
  demos/netbsd_personality_smoke.c
)

if [[ "$include_legacy_toy" == "1" ]]; then
  for src in "${legacy_toy_c[@]}"; do
    asm="/tmp/$(basename "$src" .c).s"
    "${lnp64[@]}" cc --toy-bootstrap "$src" -o "$asm"
    echo "== $src =="
    "${lnp64[@]}" run "$asm"
  done
fi

for src in demos/*.s; do
  echo "== $src =="
  "${lnp64[@]}" run "$src"
done

#!/usr/bin/env bash
set -euo pipefail

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

for src in "${legacy_toy_c[@]}"; do
  asm="/tmp/$(basename "$src" .c).s"
  "${lnp64[@]}" cc --toy-bootstrap "$src" -o "$asm"
  echo "== $src =="
  "${lnp64[@]}" run "$asm"
done

for src in demos/*.s; do
  echo "== $src =="
  "${lnp64[@]}" run "$src"
done

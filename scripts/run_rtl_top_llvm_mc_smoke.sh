#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

llvm_mc="${LLVM_MC:-target/llvm-lnp64-build/bin/llvm-mc}"
llvm_objdump="${LLVM_OBJDUMP:-target/llvm-lnp64-build/bin/llvm-objdump}"
source_asm="${1:-tests/rtl/programs/top_llvm_mc_exit.s}"

if [[ ! -x "$llvm_mc" || ! -x "$llvm_objdump" ]]; then
  printf '%s\n' "missing llvm-mc/llvm-objdump for LNP64" >&2
  printf '%s\n' "run LNP64_LLVM_DOCKER_SKIP_BUILD=1 LNP64_LLVM_GATE=mc bash scripts/run_real_llvm_lnp64_mc_docker.sh first, or set LLVM_MC/LLVM_OBJDUMP" >&2
  exit 1
fi
if [[ ! -f "$source_asm" ]]; then
  printf 'missing LLVM MC top-level source: %s\n' "$source_asm" >&2
  exit 1
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/lnp64_top_llvm_mc.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

obj="$tmp_dir/top_llvm_mc.o"
dump="$tmp_dir/top_llvm_mc.dump"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$source_asm" -o "$obj"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$obj" >"$dump"
grep -q 'exit r' "$dump"

bash scripts/run_rtl_top_program_smoke.sh "$dump"

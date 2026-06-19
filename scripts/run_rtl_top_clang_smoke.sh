#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

clang="${LLVM_CLANG:-target/llvm-lnp64-build/bin/clang}"
llvm_objdump="${LLVM_OBJDUMP:-target/llvm-lnp64-build/bin/llvm-objdump}"
source_c="${1:-tests/rtl/programs/top_clang_return_7.c}"

if [[ ! -x "$clang" || ! -x "$llvm_objdump" ]]; then
  printf '%s\n' "missing clang/llvm-objdump for LNP64" >&2
  printf '%s\n' "run LNP64_LLVM_DOCKER_SKIP_BUILD=1 bash scripts/run_real_llvm_lnp64_docker.sh first, or set LLVM_CLANG/LLVM_OBJDUMP" >&2
  exit 1
fi
if [[ ! -f "$source_c" ]]; then
  printf 'missing clang top-level source: %s\n' "$source_c" >&2
  exit 1
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/lnp64_top_clang.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

obj="$tmp_dir/top_clang.o"
dump="$tmp_dir/top_clang.dump"
hex="$tmp_dir/top_clang.hex"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables -O2 -c "$source_c" -o "$obj"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$obj" >"$dump"
grep -q '0000000000000000 <main>:' "$dump"
grep -q 'li r1, 7' "$dump"
grep -q 'ret' "$dump"
python3 scripts/llvm_objdump_to_flat_hex.py --wrap-call-exit-r1 "$dump" -o "$hex"

bash scripts/run_rtl_top_program_smoke.sh "$hex"

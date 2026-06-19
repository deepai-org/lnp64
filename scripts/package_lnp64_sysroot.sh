#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
clang="${LNP64_CLANG:-$build_dir/bin/clang}"
llvm_mc="${LNP64_LLVM_MC:-$build_dir/bin/llvm-mc}"

if [[ ! -x "$clang" ]]; then
  printf 'missing LNP64 clang: %s\n' "$clang" >&2
  exit 1
fi
if [[ ! -x "$llvm_mc" ]]; then
  printf 'missing LNP64 llvm-mc: %s\n' "$llvm_mc" >&2
  exit 1
fi

rm -rf "$sysroot"
mkdir -p "$sysroot/usr/include" "$sysroot/usr/lib/lnp64"
cp -a toolchain/include/. "$sysroot/usr/include/"
cp -a toolchain/lnp64_intrinsics.h "$sysroot/usr/lnp64_intrinsics.h"
cp -a toolchain/lnp64_static.ld "$sysroot/usr/lib/lnp64/lnp64_static.ld"

"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj \
  toolchain/crt0_lnp64.s -o "$sysroot/usr/lib/lnp64/crt0.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj \
  toolchain/liblnp64_min.s -o "$sysroot/usr/lib/lnp64/liblnp64_min.o"

compile_shim() {
  local source="$1"
  local base
  base="$(basename "$source")"
  base="${base%_min.c}"
  base="${base//_/-}"
  "$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
    -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
    -I "$sysroot/usr/include" -I toolchain \
    -c "$source" -o "$sysroot/usr/lib/lnp64/${base}-min.o"
}

for source in toolchain/liblnp64_*_min.c; do
  compile_shim "$source"
done

printf 'LNP64 sysroot packaged: %s\n' "$sysroot"

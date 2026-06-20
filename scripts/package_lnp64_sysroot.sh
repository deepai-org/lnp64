#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
clang="${LNP64_CLANG:-$build_dir/bin/clang}"
llvm_mc="${LNP64_LLVM_MC:-$build_dir/bin/llvm-mc}"
lld="${LNP64_LLD:-$build_dir/bin/ld.lld}"
smoke_dir="${LNP64_SYSROOT_SMOKE_DIR:-target/lnp64-sysroot-smoke}"
lnp64_bin="${LNP64_BIN:-${CARGO_TARGET_DIR:-target}/debug/lnp64}"

if [[ ! -x "$clang" ]]; then
  printf 'missing LNP64 clang: %s\n' "$clang" >&2
  exit 1
fi
if [[ ! -x "$llvm_mc" ]]; then
  printf 'missing LNP64 llvm-mc: %s\n' "$llvm_mc" >&2
  exit 1
fi
if [[ ! -x "$lld" ]]; then
  printf 'missing LNP64 lld: %s\n' "$lld" >&2
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

assemble_shim() {
  local source="$1"
  local base
  base="$(basename "$source")"
  base="${base%_min.s}"
  base="${base//_/-}"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj \
    "$source" -o "$sysroot/usr/lib/lnp64/${base}-min.o"
}

for source in toolchain/liblnp64_*_min.s; do
  assemble_shim "$source"
done

rm -rf "$smoke_dir"
mkdir -p "$smoke_dir"
smoke_c="$smoke_dir/sysroot-smoke.c"
cat >"$smoke_c" <<'C'
#include <unistd.h>

int main(void) {
  static const char message[] = "sysroot smoke\n";
  ssize_t written = write(STDOUT_FILENO, message, sizeof(message) - 1);
  return written == (ssize_t)(sizeof(message) - 1) ? 0 : 1;
}
C

smoke_obj="$smoke_dir/sysroot-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I "$sysroot/usr/include" -I toolchain \
  -c "$smoke_c" -o "$smoke_obj"

smoke_elf="$smoke_dir/sysroot-smoke.elf"
"$lld" -flavor gnu -static -m elf64lnp64 \
  -T "$sysroot/usr/lib/lnp64/lnp64_static.ld" \
  -o "$smoke_elf" \
  "$sysroot/usr/lib/lnp64/crt0.o" \
  "$smoke_obj" \
  "$sysroot/usr/lib/lnp64/liblnp64-fd-min.o"
test -s "$smoke_elf"

if [[ ! -x "$lnp64_bin" ]]; then
  cargo build --quiet --bin lnp64
fi
"$lnp64_bin" elf-plan "$smoke_elf" >/dev/null
smoke_output="$("$lnp64_bin" run-elf "$smoke_elf")"
grep -q 'sysroot smoke' <<<"$smoke_output"
grep -q 'exit=0' <<<"$smoke_output"
printf 'LNP64 sysroot run-elf smoke passed: %s\n' "$smoke_elf"

# Build a liblnp64.a archive from the core (non-sbase) object files
# so programs can link against a single archive without duplicate-symbol errors.
liblnp64_a="$sysroot/usr/lib/lnp64/liblnp64.a"
rm -f "$liblnp64_a"
core_objs=()
for obj in "$sysroot/usr/lib/lnp64"/liblnp64-*-min.o; do
  base=$(basename "$obj")
  # Skip sbase objects — they contain full programs with their own main-helpers
  case "$base" in liblnp64-sbase-*) continue;; esac
  core_objs+=("$obj")
done
llvm-ar rcs "$liblnp64_a" "${core_objs[@]}"
printf 'LNP64 liblnp64.a built: %d objects\n' "${#core_objs[@]}"

printf 'LNP64 sysroot packaged: %s\n' "$sysroot"

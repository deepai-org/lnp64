#!/usr/bin/env bash
set -euo pipefail

# Build and run the real SQLite 3.45.0 in-memory database test on LNP64.
# Validates CREATE TABLE, INSERT, SELECT with the real upstream amalgamation.

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
work_dir="${LNP64_SQLITE_BUILD_DIR:-target/lnp64-sqlite-build}"
clang="${LNP64_CLANG:-$build_dir/bin/clang}"
lld="${LNP64_LLD:-$build_dir/bin/ld.lld}"
lnp64_bin="${LNP64_BIN:-${CARGO_TARGET_DIR:-target}/debug/lnp64}"

require_executable() {
  if [[ ! -x "$1" ]]; then
    printf 'missing %s: %s\n' "$2" "$1" >&2
    exit 1
  fi
}

require_executable "$clang" "LNP64 clang"
require_executable "$lld" "LNP64 lld"

if [[ ! -s "$sysroot/usr/lib/lnp64/crt0.o" ]]; then
  bash scripts/package_lnp64_sysroot.sh
fi

if [[ ! -x "$lnp64_bin" ]]; then
  cargo build --quiet --bin lnp64
fi

mkdir -p "$work_dir"

lib_dir="$sysroot/usr/lib/lnp64"
linker_script="$lib_dir/lnp64_static.ld"

sqlite_flags=(
  -DSQLITE_THREADSAFE=0
  -DSQLITE_OMIT_LOAD_EXTENSION
)

compile_flags=(
  --target=lnp64-unknown-none
  -ffreestanding -fno-pic
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables
  -isystem "$sysroot/usr/include"
  -I "$root/toolchain"
  -O0
)

# Compile SQLite amalgamation (may already be done)
sqlite_obj="$work_dir/sqlite3.o"
if [[ ! -s "$sqlite_obj" ]]; then
  printf 'Compiling SQLite 3.45.0 amalgamation...\n'
  "$clang" "${compile_flags[@]}" "${sqlite_flags[@]}" \
    -c third_party/sqlite/sqlite3.c -o "$sqlite_obj"
  printf 'real LLVM LNP64 SQLite amalgamation compile passed\n'
else
  printf 'Using cached SQLite object: %s\n' "$sqlite_obj"
fi

# Compile the test driver
test_obj="$work_dir/sqlite_test_main.o"
"$clang" "${compile_flags[@]}" "${sqlite_flags[@]}" \
  -I third_party/sqlite \
  -c demos/sqlite_test_main.c -o "$test_obj"
printf 'real LLVM LNP64 SQLite test compile passed\n'

# Link
test_elf="$work_dir/sqlite_test.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$test_elf" \
  "$lib_dir/crt0.o" \
  "$test_obj" \
  "$sqlite_obj" \
  "$lib_dir/liblnp64-stdio-min.o" \
  "$lib_dir/liblnp64-alloc-min.o" \
  "$lib_dir/liblnp64-string-min.o" \
  "$lib_dir/liblnp64-convert-min.o" \
  "$lib_dir/liblnp64-startup-min.o" \
  "$lib_dir/liblnp64-signal-min.o" \
  "$lib_dir/liblnp64-fd-min.o" \
  "$lib_dir/liblnp64-errno-min.o" \
  "$lib_dir/liblnp64-time-min.o" \
  "$lib_dir/liblnp64-math-min.o" \
  "$lib_dir/liblnp64-process-min.o" \
  "$lib_dir/liblnp64-meta-min.o" \
  "$lib_dir/liblnp64-vma-min.o" \
  "$lib_dir/liblnp64-softfloat-min.o"
printf 'real LLVM LNP64 SQLite test link passed\n'

"$lnp64_bin" elf-plan "$test_elf" >/dev/null
printf 'real LLVM LNP64 SQLite elf-plan passed\n'

run_output="$("$lnp64_bin" run-elf "$test_elf")"
printf '%s\n' "$run_output"
grep -q "exit=0" <<<"$run_output"
printf '\nPhase B: SQLite 3.45.0 in-memory database VALIDATED\n'

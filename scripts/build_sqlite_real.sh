#!/usr/bin/env bash
set -euo pipefail

# Build real SQLite 3.45.0 amalgamation on LNP64.
# SQLite is a single-file C library with minimal dependencies,
# making it an excellent test for large compilation and runtime capability.

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
work_dir="${LNP64_SQLITE_REAL_BUILD_DIR:-target/lnp64-sqlite-real-build}"
sqlite_src="third_party/sqlite/sqlite3.c"
sqlite_header="third_party/sqlite/sqlite3.h"
clang="${LNP64_CLANG:-$build_dir/bin/clang}"
lld="${LNP64_LLD:-$build_dir/bin/ld.lld}"
lnp64_bin="${LNP64_BIN:-${CARGO_TARGET_DIR:-target}/debug/lnp64}"

require_executable() {
  if [[ ! -x "$1" ]]; then
    printf 'missing %s: %s\n' "$2" "$1" >&2
    exit 1
  fi
}

require_file() {
  if [[ ! -s "$1" ]]; then
    printf 'missing %s: %s\n' "$2" "$1" >&2
    exit 1
  fi
}

require_executable "$clang" "LNP64 clang"
require_executable "$lld" "LNP64 ld.lld"
require_file "$sqlite_src" "SQLite source"
require_file "$sqlite_header" "SQLite header"

if [[ ! -s "$sysroot/usr/lib/lnp64/crt0.o" ]]; then
  bash scripts/package_lnp64_sysroot.sh
fi

if [[ ! -x "$lnp64_bin" ]]; then
  cargo build --quiet --bin lnp64
fi

mkdir -p "$work_dir"

lib_dir="$sysroot/usr/lib/lnp64"
linker_script="$lib_dir/lnp64_static.ld"
crt0_obj="$lib_dir/crt0.o"

# Compile SQLite3 with minimal flags
# SQLite is a large single file (~30k lines) that exercises:
# - Long compilation times
# - Complex control flow
# - Extensive function inlining
# - Large generated code

printf 'Compiling SQLite3 amalgamation (this may take a minute)...\n'

sqlite_obj="$work_dir/sqlite3.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -isystem "$sysroot/usr/include" -I "$root/third_party/sqlite" \
  -I "$root/toolchain" \
  -DSQLITE_OMIT_LOAD_EXTENSION \
  -DSQLITE_OMIT_PRAGMA \
  -DSQLITE_OMIT_EXPLAIN \
  -DSQLITE_OMIT_TCL_VARIABLE \
  -DSQLITE_OMIT_WAL \
  -DSQLITE_OMIT_DATETIME_FUNCS \
  -DSQLITE_THREADSAFE=0 \
  -DSQLITE_TEMP_STORE=2 \
  -DSQLITE_DISABLE_LFS \
  -O0 -c "$sqlite_src" -o "$sqlite_obj"

printf 'real LLVM LNP64 sqlite3 compile passed: %s\n' "$sqlite_obj"

# Build a simple test that uses SQLite
test_c="$work_dir/sqlite_test.c"
cat >"$test_c" <<'C'
#include <stdio.h>
#include <stdlib.h>
#include "sqlite3.h"

int main(void) {
  sqlite3 *db;
  int rc;

  printf("SQLite version: %s\n", SQLITE_VERSION);

  rc = sqlite3_open(":memory:", &db);
  if (rc != SQLITE_OK) {
    printf("Failed to open database\n");
    return 1;
  }

  sqlite3_char *errmsg = 0;
  rc = sqlite3_exec(db, "CREATE TABLE test (id INTEGER, name TEXT);", 0, 0, &errmsg);
  if (rc != SQLITE_OK) {
    printf("Failed to create table: %s\n", errmsg);
    sqlite3_free(errmsg);
    sqlite3_close(db);
    return 1;
  }

  rc = sqlite3_exec(db, "INSERT INTO test VALUES (1, 'Alice');", 0, 0, &errmsg);
  if (rc != SQLITE_OK) {
    printf("Failed to insert: %s\n", errmsg);
    sqlite3_free(errmsg);
    sqlite3_close(db);
    return 1;
  }

  printf("SQLite database operations successful\n");
  sqlite3_close(db);
  printf("exit=0\n");
  return 0;
}
C

test_obj="$work_dir/sqlite_test.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -isystem "$sysroot/usr/include" \
  -I "$root/third_party/sqlite" \
  -I "$root/toolchain" \
  -O0 -c "$test_c" -o "$test_obj"

printf 'real LLVM LNP64 sqlite3 test compile passed: %s\n' "$test_obj"

# Link with minimal libc
libc_objs=(
  "$lib_dir/liblnp64-stdio-min.o"
  "$lib_dir/liblnp64-alloc-min.o"
  "$lib_dir/liblnp64-string-min.o"
  "$lib_dir/liblnp64-convert-min.o"
  "$lib_dir/liblnp64-startup-min.o"
  "$lib_dir/liblnp64-signal-min.o"
  "$lib_dir/liblnp64-fd-min.o"
  "$lib_dir/liblnp64-errno-min.o"
)

test_elf="$work_dir/sqlite_test.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$test_elf" "$crt0_obj" "$test_obj" "$sqlite_obj" "${libc_objs[@]}"

printf 'real LLVM LNP64 sqlite3 link passed: %s\n' "$test_elf"

"$lnp64_bin" elf-plan "$test_elf" >/dev/null
printf 'real LLVM LNP64 sqlite3 elf-plan passed\n'

run_output="$("$lnp64_bin" run-elf "$test_elf")"
printf '%s\n' "$run_output"
grep -q "exit=0" <<<"$run_output"
printf 'real LLVM LNP64 sqlite3 test passed\n'

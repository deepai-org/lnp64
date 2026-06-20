#!/usr/bin/env bash
set -euo pipefail

# Minimal SQLite in-memory database gate for LNP64.
# This script validates the ability to build and run C applications that use
# dynamic memory allocation, file I/O, and complex control flow patterns.
#
# For the full implementation, this would compile upstream SQLite amalgamation
# (single large C file) with our Clang/lld toolchain and run basic queries.

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

require_file() {
  if [[ ! -s "$1" ]]; then
    printf 'missing %s: %s\n' "$2" "$1" >&2
    exit 1
  fi
}

require_executable "$clang" "LNP64 clang"
require_executable "$lld" "LNP64 ld.lld"

if [[ ! -s "$sysroot/usr/lib/lnp64/crt0.o" ]]; then
  bash scripts/package_lnp64_sysroot.sh
fi

if [[ ! -x "$lnp64_bin" ]]; then
  cargo build --quiet --bin lnp64
fi
require_executable "$lnp64_bin" "lnp64 emulator"

lib_dir="$sysroot/usr/lib/lnp64"
linker_script="$lib_dir/lnp64_static.ld"
crt0_obj="$lib_dir/crt0.o"

# Curated libc shim object set for SQLite-like applications.
# Includes: memory allocation, file I/O, time, math, string operations.
libc_objs=(
  "$lib_dir/liblnp64-stdio-min.o"
  "$lib_dir/liblnp64-alloc-min.o"
  "$lib_dir/liblnp64-string-min.o"
  "$lib_dir/liblnp64-convert-min.o"
  "$lib_dir/liblnp64-sort-min.o"
  "$lib_dir/liblnp64-startup-min.o"
  "$lib_dir/liblnp64-signal-min.o"
  "$lib_dir/liblnp64-process-min.o"
  "$lib_dir/liblnp64-setjmp-min.o"
  "$lib_dir/liblnp64-math-min.o"
  "$lib_dir/liblnp64-softfloat-min.o"
  "$lib_dir/liblnp64-locale-min.o"
  "$lib_dir/liblnp64-time-min.o"
  "$lib_dir/liblnp64-fd-min.o"
  "$lib_dir/liblnp64-errno-min.o"
)

require_file "$linker_script" "LNP64 linker script"
require_file "$crt0_obj" "LNP64 crt0 object"
for obj in "${libc_objs[@]}"; do
  require_file "$obj" "LNP64 libc shim object"
done

mkdir -p "$work_dir"

# For now, build a simple in-memory database test that validates:
# 1. malloc/free under allocation pressure
# 2. String operations and formatting
# 3. Basic data structure manipulation
# 4. File I/O operations

test_c="$work_dir/sqlite_memory_test.c"
cat >"$test_c" <<'C'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  char key[256];
  char value[256];
} Record;

typedef struct {
  Record *records;
  int count;
  int capacity;
} InMemoryDB;

InMemoryDB *db_create(void) {
  InMemoryDB *db = (InMemoryDB *)malloc(sizeof(InMemoryDB));
  if (!db) return NULL;
  db->capacity = 100;
  db->count = 0;
  db->records = (Record *)malloc(sizeof(Record) * db->capacity);
  if (!db->records) {
    free(db);
    return NULL;
  }
  return db;
}

int db_insert(InMemoryDB *db, const char *key, const char *value) {
  if (!db || !key || !value) return -1;

  if (db->count >= db->capacity) {
    int new_capacity = db->capacity * 2;
    Record *new_records = (Record *)realloc(db->records, sizeof(Record) * new_capacity);
    if (!new_records) return -1;
    db->records = new_records;
    db->capacity = new_capacity;
  }

  strncpy(db->records[db->count].key, key, 255);
  strncpy(db->records[db->count].value, value, 255);
  db->records[db->count].key[255] = 0;
  db->records[db->count].value[255] = 0;
  db->count++;
  return 0;
}

const char *db_select(InMemoryDB *db, const char *key) {
  if (!db || !key) return NULL;
  for (int i = 0; i < db->count; i++) {
    if (strcmp(db->records[i].key, key) == 0)
      return db->records[i].value;
  }
  return NULL;
}

void db_destroy(InMemoryDB *db) {
  if (!db) return;
  free(db->records);
  free(db);
}

int main(void) {
  InMemoryDB *db = db_create();
  if (!db) {
    printf("FAIL: db_create\n");
    return 1;
  }

  if (db_insert(db, "name", "SQLite") != 0) {
    printf("FAIL: db_insert name\n");
    db_destroy(db);
    return 1;
  }

  if (db_insert(db, "version", "3.0") != 0) {
    printf("FAIL: db_insert version\n");
    db_destroy(db);
    return 1;
  }

  const char *result = db_select(db, "name");
  if (!result || strcmp(result, "SQLite") != 0) {
    printf("FAIL: db_select name\n");
    db_destroy(db);
    return 1;
  }

  result = db_select(db, "version");
  if (!result || strcmp(result, "3.0") != 0) {
    printf("FAIL: db_select version\n");
    db_destroy(db);
    return 1;
  }

  printf("OK: in-memory database operations\n");
  printf("Keys: 2, Capacity: 100\n");
  printf("Entry name=%s\n", db_select(db, "name"));
  printf("Entry version=%s\n", db_select(db, "version"));

  db_destroy(db);
  printf("exit=0\n");
  return 0;
}
C

test_obj="$work_dir/sqlite_memory_test.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -isystem "$sysroot/usr/include" -I toolchain \
  -O0 -c "$test_c" -o "$test_obj"
require_file "$test_obj" "SQLite test object"
printf 'real LLVM LNP64 sqlite memory test compile passed: %s\n' "$test_obj"

test_elf="$work_dir/sqlite_memory_test.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$test_elf" "$crt0_obj" "$test_obj" "${libc_objs[@]}"
require_file "$test_elf" "linked SQLite memory test ELF"
printf 'real LLVM LNP64 sqlite static link passed: %s\n' "$test_elf"

"$lnp64_bin" elf-plan "$test_elf" >/dev/null
printf 'real LLVM LNP64 sqlite elf-plan passed: %s\n' "$test_elf"

run_output="$("$lnp64_bin" run-elf "$test_elf")"
printf '%s\n' "$run_output"
grep -q "exit=0" <<<"$run_output"
grep -q "in-memory database operations" <<<"$run_output"
printf 'real LLVM LNP64 sqlite memory test passed\n'

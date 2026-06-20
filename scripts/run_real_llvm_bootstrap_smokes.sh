#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
smoke_dir="${LNP64_BOOTSTRAP_SMOKE_DIR:-target/lnp64-llvm-bootstrap-smoke}"
clang="${LNP64_CLANG:-$build_dir/bin/clang}"
llvm_objdump="${LNP64_LLVM_OBJDUMP:-$build_dir/bin/llvm-objdump}"
lld="${LNP64_LLD:-$build_dir/bin/ld.lld}"
lnp64_bin="${LNP64_BIN:-${CARGO_TARGET_DIR:-target}/debug/lnp64}"
cases="${LNP64_BOOTSTRAP_CASES:-all}"

require_executable() {
  local path="$1"
  local label="$2"
  if [[ ! -x "$path" ]]; then
    printf 'missing %s: %s\n' "$label" "$path" >&2
    exit 1
  fi
}

require_file() {
  local path="$1"
  local label="$2"
  if [[ ! -s "$path" ]]; then
    printf 'missing %s: %s\n' "$label" "$path" >&2
    exit 1
  fi
}

require_executable "$clang" "LNP64 clang"
require_executable "$llvm_objdump" "LNP64 llvm-objdump"
require_executable "$lld" "LNP64 ld.lld"

if [[ ! -s "$sysroot/usr/lib/lnp64/crt0.o" ||
      ! -s "$sysroot/usr/lib/lnp64/lnp64_static.ld" ||
      ! -s "$sysroot/usr/lib/lnp64/liblnp64-fd-min.o" ]]; then
  bash scripts/package_lnp64_sysroot.sh
fi

if [[ ! -x "$lnp64_bin" ]]; then
  cargo build --quiet --bin lnp64
fi
require_executable "$lnp64_bin" "lnp64 emulator"

mkdir -p "$smoke_dir"

linker_script="$sysroot/usr/lib/lnp64/lnp64_static.ld"
crt0_obj="$sysroot/usr/lib/lnp64/crt0.o"
lib_dir="$sysroot/usr/lib/lnp64"
common_libs=(
  "$lib_dir/liblnp64-fd-min.o"
  "$lib_dir/liblnp64-alloc-min.o"
  "$lib_dir/liblnp64-string-min.o"
  "$lib_dir/liblnp64-process-min.o"
  "$lib_dir/liblnp64-futex-min.o"
)

require_file "$linker_script" "LNP64 linker script"
require_file "$crt0_obj" "LNP64 crt0 object"
for lib in "${common_libs[@]}"; do
  require_file "$lib" "LNP64 libc shim object"
done

run_crt0_smoke() {
  local dump="$smoke_dir/crt0-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$crt0_obj" >"$dump"
  grep -q 'errno_set r0' "$dump"
  grep -q 'exit r1' "$dump"
  printf 'real LLVM bootstrap crt0 smoke passed: %s\n' "$crt0_obj"
}

compile_link_run() {
  local case_name="$1"
  local source="$2"
  local obj="$3"
  local elf="$4"
  local expected="$5"
  shift 5

  "$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
    -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
    -Wno-implicit-function-declaration -isystem "$sysroot/usr/include" \
    -I toolchain \
    -c "$source" -o "$obj"
  require_file "$obj" "$case_name object"

  local dump="$obj.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$obj" >"$dump"
  local needle
  for needle in "$@"; do
    grep -q "$needle" "$dump"
  done
  printf 'real LLVM bootstrap %s object smoke passed: %s\n' "$case_name" "$obj"

  "$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
    -o "$elf" "$crt0_obj" "$obj" "${common_libs[@]}"
  require_file "$elf" "$case_name static ELF"
  printf 'real LLVM bootstrap %s static link passed: %s\n' "$case_name" "$elf"

  "$lnp64_bin" elf-plan "$elf" >/dev/null
  printf 'real LLVM bootstrap %s elf-plan passed: %s\n' "$case_name" "$elf"

  local run_output
  run_output="$("$lnp64_bin" run-elf "$elf")"
  grep -q "$expected" <<<"$run_output"
  grep -q 'exit=0' <<<"$run_output"
  printf 'real LLVM bootstrap %s run-elf passed: %s\n' "$case_name" "$elf"
}

run_case() {
  case "$1" in
    crt0)
      run_crt0_smoke
      ;;
    hello)
      compile_link_run hello demos/hello.c \
        "$smoke_dir/hello-clang-smoke.o" \
        "$smoke_dir/lnp64-hello-clang-linked.elf" \
        'hello from LNP64' \
        'la r' 'call '
      ;;
    arithmetic|factorial)
      compile_link_run arithmetic demos/factorial.c \
        "$smoke_dir/factorial-clang-smoke.o" \
        "$smoke_dir/lnp64-factorial-clang-linked.elf" \
        'factorial ok' \
        'ld.w r' 'st.w r' 'mul r' 'cmp r' 'call '
      ;;
    memory|allocator)
      compile_link_run memory demos/allocator.c \
        "$smoke_dir/allocator-clang-smoke.o" \
        "$smoke_dir/lnp64-allocator-clang-linked.elf" \
        'alloc ok' \
        'la r' 'ld.w r' 'st.w r' 'cmp r' 'call '
      ;;
    calls|fibonacci)
      compile_link_run calls demos/fibonacci.c \
        "$smoke_dir/fibonacci-clang-smoke.o" \
        "$smoke_dir/lnp64-fibonacci-clang-linked.elf" \
        'fibonacci ok' \
        '<fib_recursive>:' '<main>:' 'add r' 'call ' 'ret'
      ;;
    *)
      printf 'unknown LNP64_BOOTSTRAP_CASES entry: %s\n' "$1" >&2
      exit 2
      ;;
  esac
}

if [[ "$cases" == "all" ]]; then
  cases="crt0 hello arithmetic memory calls"
fi
for case_name in ${cases//,/ }; do
  run_case "$case_name"
done
printf 'real LLVM bootstrap smokes passed: %s\n' "$smoke_dir"

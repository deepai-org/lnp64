#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

pick_llvm_tool() {
  local configured="$1"
  local target_path="$2"
  local build_path="$3"
  if [[ -n "$configured" ]]; then
    printf '%s\n' "$configured"
  elif [[ -x "$target_path" ]]; then
    printf '%s\n' "$target_path"
  else
    printf '%s\n' "$build_path"
  fi
}

clang="$(pick_llvm_tool "${LLVM_CLANG:-}" \
  target/llvm-lnp64-build/bin/clang build/llvm-lnp64-build/bin/clang)"
lld="$(pick_llvm_tool "${LLVM_LLD:-}" \
  target/llvm-lnp64-build/bin/lld build/llvm-lnp64-build/bin/lld)"
llvm_mc="$(pick_llvm_tool "${LLVM_MC:-}" \
  target/llvm-lnp64-build/bin/llvm-mc build/llvm-lnp64-build/bin/llvm-mc)"
source_c="${1:-tests/rtl/programs/top_linked_exit.c}"

if [[ ! -x "$clang" || ! -x "$lld" || ! -x "$llvm_mc" ]]; then
  printf '%s\n' "missing clang/lld/llvm-mc for LNP64" >&2
  printf '%s\n' "run LNP64_LLVM_DOCKER_SKIP_BUILD=1 bash scripts/run_real_llvm_lnp64_docker.sh first, or set LLVM_CLANG/LLVM_LLD/LLVM_MC" >&2
  exit 1
fi
if [[ ! -f "$source_c" ]]; then
  printf 'missing linked LLVM top-level source: %s\n' "$source_c" >&2
  exit 1
fi
if [[ -z "${LNP64_BIN:-}" ]]; then
  cargo build --quiet
  export LNP64_BIN="$root/target/debug/lnp64"
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/lnp64_top_linked_llvm.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

linker_script="$tmp_dir/lnp64_flat_top.ld"
cat >"$linker_script" <<'LD'
OUTPUT_ARCH(lnp64)
ENTRY(_start)

PHDRS
{
  text PT_LOAD FLAGS(5);
  data PT_LOAD FLAGS(6);
}

SECTIONS
{
  . = 0x1000;
  .text : ALIGN(4)
  {
    KEEP(*(.text.startup .text.startup.*))
    *(.text .text.*)
  } :text

  . = 0x10000;
  .data : ALIGN(8)
  {
    *(.data .data.*)
    *(.sdata .sdata.*)
  } :data

  .bss (NOLOAD) : ALIGN(8)
  {
    *(.bss .bss.*)
    *(COMMON)
  } :data

  /DISCARD/ :
  {
    *(.comment)
    *(.note.GNU-stack)
    *(.eh_frame .eh_frame.*)
  }
}
LD

obj="$tmp_dir/top_linked.o"
startup_obj="$tmp_dir/top_linked_startup.o"
elf="$tmp_dir/top_linked.elf"
hex="$tmp_dir/top_linked.hex"
data_hex="$tmp_dir/top_linked.data.hex"

"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -O2 \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I toolchain -c "$source_c" -o "$obj"
test -s "$obj"

link_objects=()
if ! grep -Eq '(^|[[:space:]])_start[[:space:]]*\(' "$source_c"; then
  cat >"$tmp_dir/top_linked_startup.s" <<'ASM'
.text
.globl _start
.type _start,@function
_start:
  CALL main
  EXIT r1
ASM
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj \
    "$tmp_dir/top_linked_startup.s" -o "$startup_obj"
  test -s "$startup_obj"
  link_objects+=("$startup_obj")
fi
link_objects+=("$obj")

"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" -o "$elf" "${link_objects[@]}"
test -s "$elf"

"$LNP64_BIN" elf-plan "$elf" >/dev/null
"$LNP64_BIN" elf-flat-exec "$elf" -o "$hex" --data-hex "$data_hex"
if [[ -s "$data_hex" ]]; then
  bash scripts/run_rtl_top_program_smoke.sh "$hex" "$data_hex"
else
  bash scripts/run_rtl_top_program_smoke.sh "$hex"
fi

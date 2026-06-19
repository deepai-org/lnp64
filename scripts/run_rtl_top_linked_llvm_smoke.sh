#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

clang="${LLVM_CLANG:-target/llvm-lnp64-build/bin/clang}"
lld="${LLVM_LLD:-target/llvm-lnp64-build/bin/lld}"
source_c="${1:-tests/rtl/programs/top_linked_exit.c}"

if [[ ! -x "$clang" || ! -x "$lld" ]]; then
  printf '%s\n' "missing clang/lld for LNP64" >&2
  printf '%s\n' "run LNP64_LLVM_DOCKER_SKIP_BUILD=1 bash scripts/run_real_llvm_lnp64_docker.sh first, or set LLVM_CLANG/LLVM_LLD" >&2
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
elf="$tmp_dir/top_linked.elf"
hex="$tmp_dir/top_linked.hex"
data_hex="$tmp_dir/top_linked.data.hex"

"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -O2 \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I toolchain -c "$source_c" -o "$obj"
test -s "$obj"

"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" -o "$elf" "$obj"
test -s "$elf"

"$LNP64_BIN" elf-plan "$elf" >/dev/null
"$LNP64_BIN" elf-flat-exec "$elf" -o "$hex" --data-hex "$data_hex"
if [[ -s "$data_hex" ]]; then
  printf '%s\n' "linked LLVM top smoke generated data; top-level linked data image support is not enabled for this gate yet" >&2
  exit 1
fi

bash scripts/run_rtl_top_program_smoke.sh "$hex"

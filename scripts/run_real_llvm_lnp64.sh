#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tag="${LNP64_LLVM_PROJECT_TAG:-llvmorg-14.0.6}"
project_dir="${LNP64_LLVM_PROJECT_DIR:-target/llvm-project-src}"
build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
jobs="${LNP64_LLVM_JOBS:-2}"

rewrite_with_perl() {
  local file="$1"
  shift
  local tmp
  tmp="$(mktemp)"
  cp "$file" "$tmp"
  local expr
  for expr in "$@"; do
    perl -0pi -e "$expr" "$tmp"
  done
  if cmp -s "$file" "$tmp"; then
    rm -f "$tmp"
  else
    mv "$tmp" "$file"
  fi
}

if [[ ! -d "$project_dir/.git" ]]; then
  mkdir -p "$(dirname "$project_dir")"
  git clone \
    --depth 1 \
    --filter=blob:none \
    --sparse \
    --branch "$tag" \
    https://github.com/llvm/llvm-project.git \
    "$project_dir"
  git -C "$project_dir" sparse-checkout set llvm cmake clang lld
else
  git -C "$project_dir" sparse-checkout set llvm cmake clang lld
fi

if [[ ! -f "$project_dir/llvm/CMakeLists.txt" ]]; then
  printf 'LLVM project checkout is missing llvm/CMakeLists.txt: %s\n' "$project_dir" >&2
  exit 1
fi

mkdir -p "$project_dir/llvm/lib/Target/LNP64"
cp -a llvm/lib/Target/LNP64/. "$project_dir/llvm/lib/Target/LNP64/"
mkdir -p "$project_dir/clang/lib/Basic/Targets"
cp -a clang/lib/Basic/Targets/LNP64.h "$project_dir/clang/lib/Basic/Targets/LNP64.h"
cp -a clang/lib/Basic/Targets/LNP64.cpp "$project_dir/clang/lib/Basic/Targets/LNP64.cpp"
mkdir -p "$project_dir/clang/lib/Driver/ToolChains/Arch"
cp -a clang/lib/Driver/ToolChains/Arch/LNP64.cpp \
  "$project_dir/clang/lib/Driver/ToolChains/Arch/LNP64.cpp"
mkdir -p "$project_dir/lld/ELF/Arch"
cp -a lld/ELF/Arch/LNP64.cpp "$project_dir/lld/ELF/Arch/LNP64.cpp"

llvm_cmake="$project_dir/llvm/CMakeLists.txt"
triple_h="$project_dir/llvm/include/llvm/ADT/Triple.h"
triple_cpp="$project_dir/llvm/lib/Support/Triple.cpp"
clang_basic_cmake="$project_dir/clang/lib/Basic/CMakeLists.txt"
clang_targets_cpp="$project_dir/clang/lib/Basic/Targets.cpp"
clang_driver_cmake="$project_dir/clang/lib/Driver/CMakeLists.txt"
clang_common_args_cpp="$project_dir/clang/lib/Driver/ToolChains/CommonArgs.cpp"
clang_baremetal_cpp="$project_dir/clang/lib/Driver/ToolChains/BareMetal.cpp"
lld_cmake="$project_dir/lld/ELF/CMakeLists.txt"
lld_target_h="$project_dir/lld/ELF/Target.h"
lld_target_cpp="$project_dir/lld/ELF/Target.cpp"
lld_driver_cpp="$project_dir/lld/ELF/Driver.cpp"
lld_tool_cmake="$project_dir/lld/tools/lld/CMakeLists.txt"
lld_tool_cpp="$project_dir/lld/tools/lld/lld.cpp"

if ! grep -q '^  LNP64$' "$llvm_cmake"; then
  perl -0pi -e 's/(^  Lanai\n)/$1  LNP64\n/m' "$llvm_cmake"
fi

rewrite_with_perl "$triple_h" \
  's/^    lnp64,.*\n//mg; s/^    LastArchType = .*/    lnp64,          \/\/ LNP64: LNP64 capability architecture\n    LastArchType = lnp64/m'

rewrite_with_perl "$triple_cpp" \
  's/^  case lnp64:\s+return "lnp64";\n//mg' \
  's/(StringRef Triple::getArchTypeName.*?^  case lanai:\s+return "lanai";\n)/$1  case lnp64:         return "lnp64";\n/ms' \
  's/(StringRef Triple::getArchTypePrefix.*?^  case lanai:\s+return "lanai";\n)/$1  case lnp64:      return "lnp64";\n/ms' \
  's/^  case Triple::lnp64:\n//mg' \
  's/(^  case Triple::lanai:\n)/$1  case Triple::lnp64:\n/m' \
  's/^    \.Case\("lnp64", Triple::lnp64\)\n//mg' \
  's/(^    \.Case\("lanai", Triple::lanai\)\n)/$1    .Case("lnp64", Triple::lnp64)\n/m' \
  's/^  case llvm::Triple::lnp64:\n//mg' \
  's/(^  case llvm::Triple::le64:\n)/$1  case llvm::Triple::lnp64:\n/m'

rewrite_with_perl "$clang_basic_cmake" \
  's/^  Targets\/LNP64\.cpp\n//mg; s/(^  Targets\/Lanai\.cpp\n)/$1  Targets\/LNP64.cpp\n/m'

rewrite_with_perl "$clang_targets_cpp" \
  's/^#include "Targets\/LNP64\.h"\n//mg; s/(^#include "Targets\/Lanai\.h"\n)/$1#include "Targets\/LNP64.h"\n/m' \
  's/^  case llvm::Triple::lnp64:\n    return new LNP64TargetInfo\(Triple, Opts\);\n\n//mg; s/(^  case llvm::Triple::lanai:\n    return new LanaiTargetInfo\(Triple, Opts\);\n)/$1\n  case llvm::Triple::lnp64:\n    return new LNP64TargetInfo(Triple, Opts);\n/m'

rewrite_with_perl "$clang_driver_cmake" \
  's/^  ToolChains\/Arch\/LNP64\.cpp\n//mg; s/(^  ToolChains\/Arch\/M68k\.cpp\n)/  ToolChains\/Arch\/LNP64.cpp\n$1/m'

rewrite_with_perl "$clang_common_args_cpp" \
  's/^  case llvm::Triple::lnp64:\n    return "generic-lnp64";\n\n//mg; s/(^  case llvm::Triple::lanai:\n    return getLanaiTargetCPU\(Args\);\n)/$1\n  case llvm::Triple::lnp64:\n    return "generic-lnp64";\n/m'

rewrite_with_perl "$clang_baremetal_cpp" \
  's/return isARMBareMetal\(Triple\) \|\| isAArch64BareMetal\(Triple\) \|\|\n         isRISCVBareMetal\(Triple\);/return Triple.getArch() == llvm::Triple::lnp64 || isARMBareMetal(Triple) ||\n         isAArch64BareMetal(Triple) || isRISCVBareMetal(Triple);/m'

rewrite_with_perl "$lld_cmake" \
  's/^  Arch\/LNP64\.cpp\n//mg; s/(^  Arch\/MSP430\.cpp\n)/  Arch\/LNP64.cpp\n$1/m'

rewrite_with_perl "$lld_target_h" \
  's/^TargetInfo \*getLNP64TargetInfo\(\);\n//mg; s/(^TargetInfo \*getMSP430TargetInfo\(\);\n)/TargetInfo *getLNP64TargetInfo();\n$1/m'

rewrite_with_perl "$lld_target_cpp" \
  's/^  case 0x6c64:\n    return getLNP64TargetInfo\(\);\n//mg; s/(^  case EM_MIPS:\n)/  case 0x6c64:\n    return getLNP64TargetInfo();\n$1/m'

rewrite_with_perl "$lld_driver_cpp" \
  's/^          \.Case\("elf64lnp64", \{ELF64LEKind, 0x6c64\}\)\n//mg; s/(^          \.Case\("elf64lriscv", \{ELF64LEKind, EM_RISCV\}\)\n)/$1          .Case("elf64lnp64", {ELF64LEKind, 0x6c64})\n/m'

rewrite_with_perl "$lld_tool_cmake" \
  's/target_link_libraries\(lld\n  PRIVATE\n  lldCommon\n  lldCOFF\n  lldELF\n  lldMachO\n  lldMinGW\n  lldWasm\n  \)/target_link_libraries(lld\n  PRIVATE\n  lldCommon\n  lldELF\n  )/ms' \
  's/set\(LLD_SYMLINKS_TO_CREATE\n      lld-link ld\.lld ld64\.lld wasm-ld\)/set(LLD_SYMLINKS_TO_CREATE\n      ld.lld)/m'

rewrite_with_perl "$lld_tool_cpp" \
  's/    if \(f == Gnu && isPETarget\(args\)\)\n      return mingw::link;\n    else if \(f == Gnu\)\n      return elf::link;\n    else if \(f == WinLink\)\n      return coff::link;\n    else if \(f == Darwin\)\n      return macho::link;\n    else if \(f == Wasm\)\n      return lld::wasm::link;\n    else\n      die\("lld is a generic driver\.\\n"\n          "Invoke ld\.lld \(Unix\), ld64\.lld \(macOS\), lld-link \(Windows\), wasm-ld"\n          " \(WebAssembly\) instead"\);/    if (f == Gnu)\n      return elf::link;\n    die("lld is built as an ELF-only LNP64 smoke linker; invoke ld.lld or -flavor gnu");/ms'

cmake -S "$project_dir/llvm" -B "$build_dir" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DLLVM_ENABLE_PROJECTS="clang;lld" \
  -DLLVM_TARGETS_TO_BUILD=LNP64 \
  -DLLVM_INCLUDE_TESTS=OFF \
  -DLLVM_INCLUDE_BENCHMARKS=OFF \
  -DLLVM_INCLUDE_EXAMPLES=OFF \
  -DLLVM_ENABLE_TERMINFO=OFF \
  -DLLVM_ENABLE_ZLIB=OFF \
  -DLLVM_ENABLE_LIBXML2=OFF \
  -DLLVM_ENABLE_LIBEDIT=OFF

ninja -C "$build_dir" -j "$jobs" llc llvm-mc llvm-objdump clang lld

llc="$build_dir/bin/llc"
clang="$build_dir/bin/clang"
llvm_mc="$build_dir/bin/llvm-mc"
llvm_objdump="$build_dir/bin/llvm-objdump"
lld="$build_dir/bin/lld"
"$llc" --version | sed -n '1,12p'

smoke_ir="$(mktemp)"
smoke_asm="$(mktemp)"
clang_c="$(mktemp --suffix=.c)"
main_asm="$(mktemp)"
smoke_obj="$build_dir/lnp64-smoke.o"
trap 'rm -f "$smoke_ir" "$smoke_asm" "$clang_c" "$main_asm"' EXIT

cat >"$smoke_ir" <<'IR'
define i64 @main() {
entry:
  ret i64 7
}
IR

"$llc" -mtriple=lnp64-unknown-none -verify-machineinstrs -filetype=null \
  "$smoke_ir" -o /dev/null
"$llc" -mtriple=lnp64-unknown-none "$smoke_ir" -o "$smoke_asm"
grep -q '^li r1, 7$' "$smoke_asm"
grep -q '^ret$' "$smoke_asm"
"$llc" -mtriple=lnp64-unknown-none -filetype=obj "$smoke_ir" -o "$smoke_obj"
test -s "$smoke_obj"
printf 'real LLVM LNP64 llc smoke passed: %s\n' "$smoke_obj"

cat >"$clang_c" <<'C'
int main(void) {
  return 7;
}
C

clang_obj="$build_dir/scalar-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$clang_c" -o "$clang_obj"
test -s "$clang_obj"
printf 'real LLVM LNP64 clang scalar compile smoke passed: %s\n' "$clang_obj"

hello_obj="$build_dir/hello-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/hello.c -o "$hello_obj"
test -s "$hello_obj"
hello_dump="$build_dir/hello-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$hello_obj" >"$hello_dump"
grep -q 'la r' "$hello_dump"
grep -q 'call ' "$hello_dump"
printf 'real LLVM LNP64 clang hello object smoke passed: %s\n' "$hello_obj"

factorial_obj="$build_dir/factorial-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/factorial.c -o "$factorial_obj"
test -s "$factorial_obj"
factorial_dump="$build_dir/factorial-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$factorial_obj" \
  >"$factorial_dump"
grep -q 'ld.w r' "$factorial_dump"
grep -q 'st.w r' "$factorial_dump"
grep -q 'mul r' "$factorial_dump"
grep -q 'cmp r' "$factorial_dump"
grep -q 'call ' "$factorial_dump"
printf 'real LLVM LNP64 clang factorial object smoke passed: %s\n' \
  "$factorial_obj"

crt0_obj="$build_dir/crt0-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj toolchain/crt0_lnp64.s \
  -o "$crt0_obj"
test -s "$crt0_obj"
printf 'real LLVM LNP64 llvm-mc crt0 smoke passed: %s\n' "$crt0_obj"

cat >"$main_asm" <<'ASM'
.text
.globl main
.type main,@function
main:
  li r1, 7
  ret
ASM

main_obj="$build_dir/lnp64-main-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$main_asm" \
  -o "$main_obj"
test -s "$main_obj"

crt0_dump="$build_dir/crt0-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$crt0_obj" >"$crt0_dump"
grep -q 'errno_set r0' "$crt0_dump"
grep -q 'exit r1' "$crt0_dump"
printf 'real LLVM LNP64 llvm-objdump crt0 decode smoke passed: %s\n' \
  "$crt0_dump"

linked_elf="$build_dir/lnp64-linked-smoke.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$linked_elf" "$crt0_obj" "$main_obj"
test -s "$linked_elf"
printf 'real LLVM LNP64 lld static link smoke passed: %s\n' "$linked_elf"

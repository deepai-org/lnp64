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

scalar_arith_c="$build_dir/scalar-arith-smoke.c"
cat >"$scalar_arith_c" <<'C'
volatile unsigned long scalar_input = 12345;
volatile unsigned long scalar_divisor = 37;

int main(void) {
  unsigned long x = scalar_input;
  unsigned long y = scalar_divisor;
  unsigned long v = ((x + 7) & 4095) | 16;
  v = (v ^ 85) << 2;
  v = v >> 1;
  unsigned long q = x / y;
  unsigned long r = x % y;
  long sr = (long)x % (long)y;
  return (v + q + r + (unsigned long)sr) == 391 ? 0 : 1;
}
C

scalar_arith_obj="$build_dir/scalar-arith-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$scalar_arith_c" -o "$scalar_arith_obj"
test -s "$scalar_arith_obj"
scalar_arith_dump="$build_dir/scalar-arith-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$scalar_arith_obj" \
  >"$scalar_arith_dump"
grep -q 'addi r' "$scalar_arith_dump"
grep -q 'andi r' "$scalar_arith_dump"
grep -q 'ori r' "$scalar_arith_dump"
grep -q 'xori r' "$scalar_arith_dump"
grep -q 'lsli r' "$scalar_arith_dump"
grep -q 'lsri r' "$scalar_arith_dump"
grep -q 'udiv r' "$scalar_arith_dump"
grep -q 'urem r' "$scalar_arith_dump"
grep -q 'srem r' "$scalar_arith_dump"
printf 'real LLVM LNP64 clang scalar arithmetic object smoke passed: %s\n' \
  "$scalar_arith_obj"

high_mul_c="$build_dir/high-mul-smoke.c"
cat >"$high_mul_c" <<'C'
static volatile unsigned long uhi = 0xffffffffffffffffUL;
static volatile unsigned long ulo = 2;
static volatile long sneg = -2;
static volatile long spos = 3;

__attribute__((noinline)) unsigned long umul_high(unsigned long a,
                                                  unsigned long b) {
  return ((unsigned __int128)a * (unsigned __int128)b) >> 64;
}

__attribute__((noinline)) long smul_high(long a, long b) {
  return ((__int128)a * (__int128)b) >> 64;
}

int main(void) {
  return (int)((umul_high(uhi, ulo) - 1) + (smul_high(sneg, spos) + 1));
}
C

high_mul_obj="$build_dir/high-mul-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$high_mul_c" -o "$high_mul_obj"
test -s "$high_mul_obj"
high_mul_dump="$build_dir/high-mul-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$high_mul_obj" \
  >"$high_mul_dump"
grep -q 'mulhu r' "$high_mul_dump"
grep -q 'mulh r' "$high_mul_dump"
printf 'real LLVM LNP64 clang high-multiply object smoke passed: %s\n' \
  "$high_mul_obj"

scalar_extend_c="$build_dir/scalar-extend-smoke.c"
cat >"$scalar_extend_c" <<'C'
static volatile unsigned long minus_two = 0UL - 2;
static volatile unsigned long minus_sixteen = 0UL - 16;

__attribute__((noinline)) unsigned long zext_byte(unsigned long x) {
  return (unsigned char)x;
}

__attribute__((noinline)) unsigned long zext_half(unsigned long x) {
  return (unsigned short)x;
}

__attribute__((noinline)) unsigned long zext_word(unsigned long x) {
  return (unsigned int)x;
}

__attribute__((noinline)) long sext_byte(unsigned long x) {
  return (signed char)x;
}

__attribute__((noinline)) long sext_half(unsigned long x) {
  return (short)x;
}

__attribute__((noinline)) long sext_word(unsigned long x) {
  return (int)x;
}

int main(void) {
  unsigned long zb = zext_byte(minus_two);
  unsigned long zh = zext_half(minus_two);
  unsigned long zw = zext_word(minus_sixteen);
  long sb = sext_byte(minus_two);
  long sh = sext_half(minus_two);
  long sw = sext_word(minus_sixteen);

  return (int)((zb >> 8) + ((zb + 2) & 0xff) + (zh >> 16) +
               ((zh + 2) & 0xffff) + (zw >> 32) + ((zw + 16) & 0xff) +
               (sb + 2) + (sh + 2) + (sw + 16));
}
C

scalar_extend_obj="$build_dir/scalar-extend-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$scalar_extend_c" -o "$scalar_extend_obj"
test -s "$scalar_extend_obj"
scalar_extend_dump="$build_dir/scalar-extend-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$scalar_extend_obj" \
  >"$scalar_extend_dump"
grep -q 'zext.b r' "$scalar_extend_dump"
grep -q 'zext.h r' "$scalar_extend_dump"
grep -q 'zext.w r' "$scalar_extend_dump"
grep -q 'sext.b r' "$scalar_extend_dump"
grep -q 'sext.h r' "$scalar_extend_dump"
grep -q 'sext.w r' "$scalar_extend_dump"
printf 'real LLVM LNP64 clang scalar extension object smoke passed: %s\n' \
  "$scalar_extend_obj"

bitmanip_c="$build_dir/bitmanip-smoke.c"
cat >"$bitmanip_c" <<'C'
static volatile unsigned long clz_input = 0x00f0000000000000UL;
static volatile unsigned long ctz_input = 0x10UL;
static volatile unsigned long pop_input = 0xf0f0UL;
static volatile unsigned long bswap_input = 0x0123456789abcdefUL;
static volatile unsigned long rol_input = 1UL;
static volatile unsigned long ror_input = 0x100UL;
static volatile unsigned long rotate_count = 8UL;

__attribute__((noinline)) unsigned long clz64(unsigned long x) {
  return __builtin_clzll(x);
}

__attribute__((noinline)) unsigned long ctz64(unsigned long x) {
  return __builtin_ctzll(x);
}

__attribute__((noinline)) unsigned long pop64(unsigned long x) {
  return __builtin_popcountll(x);
}

__attribute__((noinline)) unsigned long rol64(unsigned long x, unsigned long n) {
  n &= 63;
  return (x << n) | (x >> ((64 - n) & 63));
}

__attribute__((noinline)) unsigned long ror64(unsigned long x, unsigned long n) {
  n &= 63;
  return (x >> n) | (x << ((64 - n) & 63));
}

__attribute__((noinline)) unsigned long bswap64(unsigned long x) {
  return __builtin_bswap64(x);
}

int main(void) {
  return (int)((clz64(clz_input) - 8) + (ctz64(ctz_input) - 4) +
               (pop64(pop_input) - 8) + ((rol64(rol_input, rotate_count) >> 8) - 1) +
               (ror64(ror_input, rotate_count) - 1) +
               ((bswap64(bswap_input) & 0xff) - 1));
}
C

bitmanip_obj="$build_dir/bitmanip-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$bitmanip_c" -o "$bitmanip_obj"
test -s "$bitmanip_obj"
bitmanip_dump="$build_dir/bitmanip-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$bitmanip_obj" \
  >"$bitmanip_dump"
grep -q 'clz r' "$bitmanip_dump"
grep -q 'ctz r' "$bitmanip_dump"
grep -q 'popcnt r' "$bitmanip_dump"
grep -q 'rol r' "$bitmanip_dump"
grep -q 'ror r' "$bitmanip_dump"
grep -q 'bswap64 r' "$bitmanip_dump"
printf 'real LLVM LNP64 clang bit-manip object smoke passed: %s\n' \
  "$bitmanip_obj"

csel_c="$build_dir/csel-smoke.c"
cat >"$csel_c" <<'C'
static volatile long low = 3;
static volatile long high = 7;
static volatile long true_value = 11;
static volatile long false_value = 19;

int main(void) {
  long lo = low;
  long hi = high;
  long tv = true_value;
  long fv = false_value;
  long gt_true = hi > lo ? tv : fv;
  long gt_false = lo > hi ? tv : fv;
  unsigned long ulo = (unsigned long)lo;
  unsigned long uhi = (unsigned long)hi;
  unsigned long utv = (unsigned long)tv;
  unsigned long ufv = (unsigned long)fv;
  unsigned long ult_true = ulo < uhi ? utv : ufv;
  unsigned long ult_false = uhi < ulo ? utv : ufv;

  return (int)((gt_true - tv) + (gt_false - fv) + (ult_true - utv) +
               (ult_false - ufv));
}
C

csel_obj="$build_dir/csel-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$csel_c" -o "$csel_obj"
test -s "$csel_obj"
csel_dump="$build_dir/csel-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$csel_obj" >"$csel_dump"
grep -q 'csel.gt r' "$csel_dump"
grep -q 'csel.ult r' "$csel_dump"
printf 'real LLVM LNP64 clang csel object smoke passed: %s\n' "$csel_obj"

call_clobber_c="$build_dir/call-clobber-smoke.c"
cat >"$call_clobber_c" <<'C'
__attribute__((noinline)) unsigned long low8(unsigned long x) {
  return (unsigned char)x;
}

__attribute__((noinline)) unsigned long low16(unsigned long x) {
  return (unsigned short)x;
}

int main(void) {
  return (int)((low8(0UL - 2) - 254) + (low16(0UL - 2) - 65534));
}
C

call_clobber_obj="$build_dir/call-clobber-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$call_clobber_c" -o "$call_clobber_obj"
test -s "$call_clobber_obj"
call_clobber_dump="$build_dir/call-clobber-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$call_clobber_obj" \
  >"$call_clobber_dump"
grep -q 'call ' "$call_clobber_dump"
grep -q 'zext.b r' "$call_clobber_dump"
grep -q 'zext.h r' "$call_clobber_dump"
printf 'real LLVM LNP64 clang call-clobber object smoke passed: %s\n' \
  "$call_clobber_obj"

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

allocator_obj="$build_dir/allocator-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/allocator.c -o "$allocator_obj"
test -s "$allocator_obj"
allocator_dump="$build_dir/allocator-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$allocator_obj" \
  >"$allocator_dump"
grep -q 'la r' "$allocator_dump"
grep -q 'ld.w r' "$allocator_dump"
grep -q 'st.w r' "$allocator_dump"
grep -q 'cmp r' "$allocator_dump"
grep -q 'call ' "$allocator_dump"
printf 'real LLVM LNP64 clang allocator object smoke passed: %s\n' \
  "$allocator_obj"

fibonacci_obj="$build_dir/fibonacci-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/fibonacci.c -o "$fibonacci_obj"
test -s "$fibonacci_obj"
fibonacci_dump="$build_dir/fibonacci-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$fibonacci_obj" \
  >"$fibonacci_dump"
grep -q '<fib_recursive>:' "$fibonacci_dump"
grep -q '<main>:' "$fibonacci_dump"
grep -q 'add r' "$fibonacci_dump"
grep -q 'call ' "$fibonacci_dump"
grep -q 'ret' "$fibonacci_dump"
printf 'real LLVM LNP64 clang fibonacci object smoke passed: %s\n' \
  "$fibonacci_obj"

indirect_call_c="$build_dir/indirect-call-smoke.c"
cat >"$indirect_call_c" <<'C'
int add3(int x) {
  return x + 3;
}

int call_it(int (*fn)(int), int value) {
  return fn(value);
}

int main(void) {
  return call_it(add3, 4) - 7;
}
C

indirect_call_obj="$build_dir/indirect-call-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$indirect_call_c" -o "$indirect_call_obj"
test -s "$indirect_call_obj"
indirect_call_dump="$build_dir/indirect-call-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$indirect_call_obj" \
  >"$indirect_call_dump"
grep -q 'call_reg' "$indirect_call_dump"
printf 'real LLVM LNP64 clang indirect call object smoke passed: %s\n' \
  "$indirect_call_obj"

intrinsic_push_c="$build_dir/intrinsic-push.c"
cat >"$intrinsic_push_c" <<'C'
#include "lnp64_intrinsics.h"
int main(void) {
  return __lnp_push(1, (lnp64_word_t)"intrinsic push ok\n", 18) - 18;
}
C

intrinsic_push_obj="$build_dir/intrinsic-push-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_push_c" -o "$intrinsic_push_obj"
test -s "$intrinsic_push_obj"
intrinsic_push_dump="$build_dir/intrinsic-push-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_push_obj" \
  >"$intrinsic_push_dump"
grep -q 'push r' "$intrinsic_push_dump"
printf 'real LLVM LNP64 clang intrinsic push object smoke passed: %s\n' \
  "$intrinsic_push_obj"

intrinsic_await_c="$build_dir/intrinsic-await.c"
cat >"$intrinsic_await_c" <<'C'
#include "lnp64_intrinsics.h"
int main(void) {
  return __lnp_await(0, 0, 0);
}
C

intrinsic_await_obj="$build_dir/intrinsic-await-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_await_c" -o "$intrinsic_await_obj"
test -s "$intrinsic_await_obj"
intrinsic_await_dump="$build_dir/intrinsic-await-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_await_obj" \
  >"$intrinsic_await_dump"
grep -q 'await r' "$intrinsic_await_dump"
printf 'real LLVM LNP64 clang intrinsic await object smoke passed: %s\n' \
  "$intrinsic_await_obj"

intrinsic_call_c="$build_dir/intrinsic-call.c"
cat >"$intrinsic_call_c" <<'C'
#include "lnp64_intrinsics.h"
int main(void) {
  if (__lnp_call(0, 1, 2) != (lnp64_word_t)-1)
    return 1;
  return 0;
}
C

intrinsic_call_obj="$build_dir/intrinsic-call-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_call_c" -o "$intrinsic_call_obj"
test -s "$intrinsic_call_obj"
intrinsic_call_dump="$build_dir/intrinsic-call-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_call_obj" \
  >"$intrinsic_call_dump"
grep -q 'gate_call r' "$intrinsic_call_dump"
printf 'real LLVM LNP64 clang intrinsic call object smoke passed: %s\n' \
  "$intrinsic_call_obj"

intrinsic_gate_return_c="$build_dir/intrinsic-gate-return.c"
cat >"$intrinsic_gate_return_c" <<'C'
#include "lnp64_intrinsics.h"
int main(void) {
  if (__lnp_gate_return(1, 2, 0) != (lnp64_word_t)-1)
    return 1;
  return 0;
}
C

intrinsic_gate_return_obj="$build_dir/intrinsic-gate-return-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_gate_return_c" -o "$intrinsic_gate_return_obj"
test -s "$intrinsic_gate_return_obj"
intrinsic_gate_return_dump="$build_dir/intrinsic-gate-return-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_gate_return_obj" \
  >"$intrinsic_gate_return_dump"
grep -q 'gate_return r' "$intrinsic_gate_return_dump"
printf 'real LLVM LNP64 clang intrinsic gate return object smoke passed: %s\n' \
  "$intrinsic_gate_return_obj"

intrinsic_ctl_c="$build_dir/intrinsic-control.c"
cat >"$intrinsic_ctl_c" <<'C'
#include "lnp64_intrinsics.h"
int main(void) {
  lnp64_word_t record[1];
  record[0] = 99;
  if (__lnp_object_ctl((lnp64_word_t)record) != (lnp64_word_t)-1)
    return 1;
  if (__lnp_domain_ctl((lnp64_word_t)record) != (lnp64_word_t)-1)
    return 2;
  return 0;
}
C

intrinsic_ctl_obj="$build_dir/intrinsic-control-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_ctl_c" -o "$intrinsic_ctl_obj"
test -s "$intrinsic_ctl_obj"
intrinsic_ctl_dump="$build_dir/intrinsic-control-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_ctl_obj" \
  >"$intrinsic_ctl_dump"
grep -q 'object_ctl r' "$intrinsic_ctl_dump"
grep -q 'domain_ctl r' "$intrinsic_ctl_dump"
printf 'real LLVM LNP64 clang intrinsic control object smoke passed: %s\n' \
  "$intrinsic_ctl_obj"

inline_asm_c="$build_dir/inline-asm-smoke.c"
cat >"$inline_asm_c" <<'C'
unsigned long twice(unsigned long x) {
  __asm__ volatile ("add %0, %0, %0" : "+r"(x));
  return x;
}

int main(void) {
  return (int)twice(7) - 14;
}
C

inline_asm_obj="$build_dir/inline-asm-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$inline_asm_c" -o "$inline_asm_obj"
test -s "$inline_asm_obj"
inline_asm_dump="$build_dir/inline-asm-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$inline_asm_obj" \
  >"$inline_asm_dump"
grep -q 'add r' "$inline_asm_dump"
printf 'real LLVM LNP64 clang inline asm object smoke passed: %s\n' \
  "$inline_asm_obj"

exit_c="$build_dir/exit-smoke.c"
cat >"$exit_c" <<'C'
int main(void) {
  _exit(0);
  return 7;
}
C

exit_obj="$build_dir/exit-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c "$exit_c" -o "$exit_obj"
test -s "$exit_obj"
exit_dump="$build_dir/exit-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$exit_obj" >"$exit_dump"
grep -q 'call ' "$exit_dump"
printf 'real LLVM LNP64 clang exit object smoke passed: %s\n' "$exit_obj"

argc_c="$build_dir/argc-smoke.c"
cat >"$argc_c" <<'C'
int main(int argc, char **argv) {
  (void)argv;
  return argc;
}
C

argc_obj="$build_dir/argc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$argc_c" -o "$argc_obj"
test -s "$argc_obj"
argc_dump="$build_dir/argc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$argc_obj" >"$argc_dump"
grep -q '<main>:' "$argc_dump"
grep -q 'ret' "$argc_dump"
printf 'real LLVM LNP64 clang argc object smoke passed: %s\n' "$argc_obj"

compare_c="$build_dir/compare-smoke.c"
cat >"$compare_c" <<'C'
int same(long a, long b) {
  return a == b;
}

int different(long a, long b) {
  return a != b;
}

int less(long a, long b) {
  return a < b;
}

int main(void) {
  return same(9, 9) + different(9, 7) + less(3, 4) - 3;
}
C

compare_obj="$build_dir/compare-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$compare_c" -o "$compare_obj"
test -s "$compare_obj"
compare_dump="$build_dir/compare-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$compare_obj" \
  >"$compare_dump"
grep -q 'cset.eq' "$compare_dump"
grep -q 'cset.ne' "$compare_dump"
grep -q 'cset.lt' "$compare_dump"
printf 'real LLVM LNP64 clang comparison object smoke passed: %s\n' \
  "$compare_obj"

unsigned_compare_c="$build_dir/unsigned-compare-smoke.c"
cat >"$unsigned_compare_c" <<'C'
int below(unsigned long a, unsigned long b) {
  return a < b;
}

int above(unsigned long a, unsigned long b) {
  return a > b;
}

int below_or_equal(unsigned long a, unsigned long b) {
  return a <= b;
}

int above_or_equal(unsigned long a, unsigned long b) {
  return a >= b;
}

int branch_below(unsigned long a, unsigned long b) {
  if (a < b)
    return 0;
  return 9;
}

int main(void) {
  return below(3, 4) + above(5, 4) + below_or_equal(4, 4) +
         above_or_equal(4, 4) + branch_below(3, 4) - 4;
}
C

unsigned_compare_obj="$build_dir/unsigned-compare-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$unsigned_compare_c" -o "$unsigned_compare_obj"
test -s "$unsigned_compare_obj"
unsigned_compare_dump="$build_dir/unsigned-compare-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$unsigned_compare_obj" \
  >"$unsigned_compare_dump"
grep -q 'cmpu r' "$unsigned_compare_dump"
grep -q 'cset.ult' "$unsigned_compare_dump"
grep -q 'cset.ugt' "$unsigned_compare_dump"
grep -q 'cset.ule' "$unsigned_compare_dump"
grep -q 'cset.uge' "$unsigned_compare_dump"
grep -q 'bge ' "$unsigned_compare_dump"
printf 'real LLVM LNP64 clang unsigned comparison object smoke passed: %s\n' \
  "$unsigned_compare_obj"

signed_load_c="$build_dir/signed-load-smoke.c"
cat >"$signed_load_c" <<'C'
signed char global_byte = -2;
short global_half = -3;
volatile signed char local_byte_source = -4;
volatile short local_half_source = -5;

int load_signed(void) {
  signed char local_byte = local_byte_source;
  short local_half = local_half_source;
  return global_byte + global_half + local_byte + local_half;
}

int main(void) {
  return load_signed() + 14;
}
C

signed_load_obj="$build_dir/signed-load-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$signed_load_c" -o "$signed_load_obj"
test -s "$signed_load_obj"
signed_load_dump="$build_dir/signed-load-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$signed_load_obj" \
  >"$signed_load_dump"
grep -q 'ld.b r' "$signed_load_dump"
grep -q 'ld.h r' "$signed_load_dump"
grep -q 'sext.b r' "$signed_load_dump"
grep -q 'sext.h r' "$signed_load_dump"
printf 'real LLVM LNP64 clang signed-load object smoke passed: %s\n' \
  "$signed_load_obj"

wide_const_c="$build_dir/wide-const-smoke.c"
cat >"$wide_const_c" <<'C'
unsigned int wide_constant(void) {
  return 65531u;
}

int main(void) {
  return (int)(wide_constant() - 65531u);
}
C

wide_const_obj="$build_dir/wide-const-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$wide_const_c" -o "$wide_const_obj"
test -s "$wide_const_obj"
wide_const_dump="$build_dir/wide-const-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$wide_const_obj" \
  >"$wide_const_dump"
grep -q 'li32 r' "$wide_const_dump"
printf 'real LLVM LNP64 clang wide-constant object smoke passed: %s\n' \
  "$wide_const_obj"

stack_aggregate_c="$build_dir/stack-aggregate-smoke.c"
cat >"$stack_aggregate_c" <<'C'
struct Pair {
  int a;
  int b;
};

int stack_bytes(void) {
  unsigned char bytes[3];
  bytes[0] = 3;
  bytes[1] = 4;
  bytes[2] = 5;
  return bytes[0] + bytes[1] + bytes[2];
}

int stack_pair(void) {
  struct Pair p = {4, 6};
  return p.a + p.b;
}

int main(void) {
  return stack_bytes() + stack_pair() - 22;
}
C

stack_aggregate_obj="$build_dir/stack-aggregate-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$stack_aggregate_c" -o "$stack_aggregate_obj"
test -s "$stack_aggregate_obj"
stack_aggregate_dump="$build_dir/stack-aggregate-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$stack_aggregate_obj" \
  >"$stack_aggregate_dump"
grep -q 'add r.*r31' "$stack_aggregate_dump"
grep -q 'st.b' "$stack_aggregate_dump"
printf 'real LLVM LNP64 clang stack aggregate object smoke passed: %s\n' \
  "$stack_aggregate_obj"

libc_string_c="$build_dir/libc-string-smoke.c"
cat >"$libc_string_c" <<'C'
typedef unsigned long size_t;

size_t strlen(const char *s);
int memcmp(const void *lhs, const void *rhs, size_t len);
void *memcpy(void *dst, const void *src, size_t len);
void *memmove(void *dst, const void *src, size_t len);
void *memset(void *dst, int value, size_t len);

int main(void) {
  char src[8];
  char dst[8];
  if (memset(src, 'A', 7) != src)
    return 1;
  src[7] = 0;
  if (strlen(src) != 7)
    return 2;
  if (memcpy(dst, src, 8) != dst)
    return 3;
  if (strlen(dst) != 7)
    return 4;
  if (dst[3] != 'A')
    return 5;
  char overlap[8] = {'a', 'b', 'c', 'd', 'e', 'f', 0, 0};
  if (memmove(overlap + 2, overlap, 4) != overlap + 2)
    return 6;
  if (overlap[0] != 'a' || overlap[1] != 'b' || overlap[2] != 'a')
    return 7;
  if (overlap[3] != 'b' || overlap[4] != 'c' || overlap[5] != 'd')
    return 8;
  if (memmove(overlap, overlap + 2, 4) != overlap)
    return 9;
  if (overlap[0] != 'a' || overlap[1] != 'b' || overlap[2] != 'c')
    return 10;
  if (overlap[3] != 'd')
    return 11;
  if (memcmp(overlap, "abcd", 4) != 0)
    return 12;
  if (memcmp(overlap, "abce", 4) >= 0)
    return 13;
  if (memcmp("abce", overlap, 4) <= 0)
    return 14;
  if (memcmp(overlap, "zz", 0) != 0)
    return 15;
  return 0;
}
C

libc_string_obj="$build_dir/libc-string-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_string_c" -o "$libc_string_obj"
test -s "$libc_string_obj"
libc_string_dump="$build_dir/libc-string-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_string_obj" \
  >"$libc_string_dump"
grep -q 'call ' "$libc_string_dump"
grep -q 'sext.w' "$libc_string_dump"
printf 'real LLVM LNP64 clang minilibc string object smoke passed: %s\n' \
  "$libc_string_obj"

calloc_c="$build_dir/calloc-smoke.c"
cat >"$calloc_c" <<'C'
typedef unsigned long size_t;

void *calloc(size_t count, size_t size);
void free(void *ptr);

int main(void) {
  unsigned char *bytes = calloc(4, 2);
  if (!bytes)
    return 1;
  for (size_t i = 0; i < 8; i = i + 1) {
    if (bytes[i] != 0)
      return 2;
  }
  bytes[3] = 9;
  if (bytes[3] != 9)
    return 3;
  free(bytes);
  return 0;
}
C

calloc_obj="$build_dir/calloc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$calloc_c" -o "$calloc_obj"
test -s "$calloc_obj"
calloc_dump="$build_dir/calloc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$calloc_obj" \
  >"$calloc_dump"
grep -q 'call ' "$calloc_dump"
printf 'real LLVM LNP64 clang calloc object smoke passed: %s\n' \
  "$calloc_obj"

realloc_c="$build_dir/realloc-smoke.c"
cat >"$realloc_c" <<'C'
typedef unsigned long size_t;

void *malloc(size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);

int main(void) {
  unsigned char *bytes = malloc(4);
  if (!bytes)
    return 1;
  bytes[0] = 1;
  bytes[1] = 2;
  bytes[2] = 3;
  bytes[3] = 4;

  unsigned char *grown = realloc(bytes, 8);
  if (!grown)
    return 2;
  for (size_t i = 0; i < 4; i = i + 1) {
    if (grown[i] != i + 1)
      return 3;
  }
  grown[4] = 5;
  grown[5] = 6;
  grown[6] = 7;
  grown[7] = 8;

  unsigned char *shrunk = realloc(grown, 2);
  if (!shrunk)
    return 4;
  if (shrunk[0] != 1 || shrunk[1] != 2)
    return 5;
  free(shrunk);

  unsigned char *fresh = realloc(0, 3);
  if (!fresh)
    return 6;
  fresh[0] = 9;
  if (fresh[0] != 9)
    return 7;
  free(fresh);

  unsigned char *zero = malloc(1);
  if (!zero)
    return 8;
  if (realloc(zero, 0) != 0)
    return 9;
  return 0;
}
C

realloc_obj="$build_dir/realloc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$realloc_c" -o "$realloc_obj"
test -s "$realloc_obj"
realloc_dump="$build_dir/realloc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$realloc_obj" \
  >"$realloc_dump"
grep -q 'call ' "$realloc_dump"
printf 'real LLVM LNP64 clang realloc object smoke passed: %s\n' \
  "$realloc_obj"

read_c="$build_dir/read-smoke.c"
cat >"$read_c" <<'C'
typedef unsigned long size_t;

long read(int fd, void *buf, size_t len);

int main(void) {
  char byte = 0;
  return read(0, &byte, 0);
}
C

read_obj="$build_dir/read-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$read_c" -o "$read_obj"
test -s "$read_obj"
read_dump="$build_dir/read-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$read_obj" \
  >"$read_dump"
grep -q 'call ' "$read_dump"
printf 'real LLVM LNP64 clang read object smoke passed: %s\n' \
  "$read_obj"

stack_arg_formal_c="$build_dir/stack-arg-formal-negative.c"
cat >"$stack_arg_formal_c" <<'C'
int sum7(int a, int b, int c, int d, int e, int f, int g) {
  return a + b + c + d + e + f + g;
}
C

if "$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$stack_arg_formal_c" -o "$build_dir/stack-arg-formal-negative.o" \
  2>"$build_dir/stack-arg-formal-negative.err"; then
  printf 'expected LNP64 stack formal argument rejection to fail\n' >&2
  exit 1
fi
grep -q 'LNP64 stack formal arguments are not implemented yet' \
  "$build_dir/stack-arg-formal-negative.err"

stack_arg_call_c="$build_dir/stack-arg-call-negative.c"
cat >"$stack_arg_call_c" <<'C'
extern int sum7(int a, int b, int c, int d, int e, int f, int g);
int main(void) {
  return sum7(1, 2, 3, 4, 5, 6, 7);
}
C

if "$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$stack_arg_call_c" -o "$build_dir/stack-arg-call-negative.o" \
  2>"$build_dir/stack-arg-call-negative.err"; then
  printf 'expected LNP64 stack call argument rejection to fail\n' >&2
  exit 1
fi
grep -q 'LNP64 stack call arguments are not implemented yet' \
  "$build_dir/stack-arg-call-negative.err"
printf 'real LLVM LNP64 stack argument negative smokes passed\n'

crt0_obj="$build_dir/crt0-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj toolchain/crt0_lnp64.s \
  -o "$crt0_obj"
test -s "$crt0_obj"
printf 'real LLVM LNP64 llvm-mc crt0 smoke passed: %s\n' "$crt0_obj"

minilibc_obj="$build_dir/liblnp64-min-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj toolchain/liblnp64_min.s \
  -o "$minilibc_obj"
test -s "$minilibc_obj"
printf 'real LLVM LNP64 llvm-mc minilibc smoke passed: %s\n' "$minilibc_obj"

high_mul_asm="$build_dir/high-mul-mc-smoke.s"
cat >"$high_mul_asm" <<'ASM'
  .text
  .globl _start
_start:
  mulh r1, r2, r3
  mulhu r4, r5, r6
  mulhsu r7, r8, r9
  ret
ASM
high_mul_mc_obj="$build_dir/high-mul-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$high_mul_asm" \
  -o "$high_mul_mc_obj"
test -s "$high_mul_mc_obj"
high_mul_mc_dump="$build_dir/high-mul-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$high_mul_mc_obj" \
  >"$high_mul_mc_dump"
grep -q 'mulh r1, r2, r3' "$high_mul_mc_dump"
grep -q 'mulhu r4, r5, r6' "$high_mul_mc_dump"
grep -q 'mulhsu r7, r8, r9' "$high_mul_mc_dump"
printf 'real LLVM LNP64 llvm-mc high-multiply smoke passed: %s\n' \
  "$high_mul_mc_obj"

minilibc_dump="$build_dir/liblnp64-min-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$minilibc_obj" \
  >"$minilibc_dump"
grep -q 'pull r1, r1, r2, r3' "$minilibc_dump"
grep -q 'alloc r1, r1' "$minilibc_dump"
grep -q 'alloc_size r3, r2' "$minilibc_dump"
grep -q 'free r1' "$minilibc_dump"
printf 'real LLVM LNP64 llvm-objdump minilibc native decode smoke passed: %s\n' \
  "$minilibc_dump"

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

heap_asm="$build_dir/native-heap-smoke.s"
cat >"$heap_asm" <<'ASM'
.text
.globl main
.type main,@function
main:
  li r1, 32
  li r2, 16
  alloc_ex r3, r1, r2
  alloc_size r4, r3
  sub r1, r4, r1
  free r3
  ret
ASM

heap_obj="$build_dir/native-heap-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$heap_asm" \
  -o "$heap_obj"
test -s "$heap_obj"
heap_dump="$build_dir/native-heap-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$heap_obj" >"$heap_dump"
grep -q 'alloc_ex r3, r1, r2' "$heap_dump"
grep -q 'alloc_size r4, r3' "$heap_dump"
grep -q 'free r3' "$heap_dump"
printf 'real LLVM LNP64 native heap opcode smoke passed: %s\n' "$heap_obj"

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

heap_elf="$build_dir/lnp64-native-heap-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$heap_elf" "$crt0_obj" "$heap_obj"
test -s "$heap_elf"
printf 'real LLVM LNP64 lld native heap link smoke passed: %s\n' "$heap_elf"

intrinsic_push_elf="$build_dir/lnp64-intrinsic-push-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_push_elf" "$crt0_obj" "$intrinsic_push_obj"
test -s "$intrinsic_push_elf"
printf 'real LLVM LNP64 lld intrinsic push link smoke passed: %s\n' \
  "$intrinsic_push_elf"

intrinsic_await_elf="$build_dir/lnp64-intrinsic-await-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_await_elf" "$crt0_obj" "$intrinsic_await_obj"
test -s "$intrinsic_await_elf"
printf 'real LLVM LNP64 lld intrinsic await link smoke passed: %s\n' \
  "$intrinsic_await_elf"

intrinsic_call_elf="$build_dir/lnp64-intrinsic-call-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_call_elf" "$crt0_obj" "$intrinsic_call_obj"
test -s "$intrinsic_call_elf"
printf 'real LLVM LNP64 lld intrinsic call link smoke passed: %s\n' \
  "$intrinsic_call_elf"

intrinsic_gate_return_elf="$build_dir/lnp64-intrinsic-gate-return-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_gate_return_elf" "$crt0_obj" "$intrinsic_gate_return_obj"
test -s "$intrinsic_gate_return_elf"
printf 'real LLVM LNP64 lld intrinsic gate return link smoke passed: %s\n' \
  "$intrinsic_gate_return_elf"

intrinsic_ctl_elf="$build_dir/lnp64-intrinsic-control-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_ctl_elf" "$crt0_obj" "$intrinsic_ctl_obj"
test -s "$intrinsic_ctl_elf"
printf 'real LLVM LNP64 lld intrinsic control link smoke passed: %s\n' \
  "$intrinsic_ctl_elf"

inline_asm_elf="$build_dir/lnp64-inline-asm-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$inline_asm_elf" "$crt0_obj" "$inline_asm_obj"
test -s "$inline_asm_elf"
printf 'real LLVM LNP64 lld inline asm link smoke passed: %s\n' \
  "$inline_asm_elf"

exit_elf="$build_dir/lnp64-exit-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$exit_elf" "$crt0_obj" "$exit_obj" "$minilibc_obj"
test -s "$exit_elf"
printf 'real LLVM LNP64 lld exit link smoke passed: %s\n' "$exit_elf"

argc_elf="$build_dir/lnp64-argc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$argc_elf" "$crt0_obj" "$argc_obj"
test -s "$argc_elf"
printf 'real LLVM LNP64 lld argc link smoke passed: %s\n' "$argc_elf"

scalar_arith_elf="$build_dir/lnp64-scalar-arith-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$scalar_arith_elf" "$crt0_obj" "$scalar_arith_obj"
test -s "$scalar_arith_elf"
printf 'real LLVM LNP64 lld scalar arithmetic link smoke passed: %s\n' \
  "$scalar_arith_elf"

high_mul_elf="$build_dir/lnp64-high-mul-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$high_mul_elf" "$crt0_obj" "$high_mul_obj"
test -s "$high_mul_elf"
printf 'real LLVM LNP64 lld high-multiply link smoke passed: %s\n' \
  "$high_mul_elf"

scalar_extend_elf="$build_dir/lnp64-scalar-extend-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$scalar_extend_elf" "$crt0_obj" "$scalar_extend_obj"
test -s "$scalar_extend_elf"
printf 'real LLVM LNP64 lld scalar extension link smoke passed: %s\n' \
  "$scalar_extend_elf"

bitmanip_elf="$build_dir/lnp64-bitmanip-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$bitmanip_elf" "$crt0_obj" "$bitmanip_obj"
test -s "$bitmanip_elf"
printf 'real LLVM LNP64 lld bit-manip link smoke passed: %s\n' \
  "$bitmanip_elf"

csel_elf="$build_dir/lnp64-csel-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$csel_elf" "$crt0_obj" "$csel_obj"
test -s "$csel_elf"
printf 'real LLVM LNP64 lld csel link smoke passed: %s\n' "$csel_elf"

call_clobber_elf="$build_dir/lnp64-call-clobber-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$call_clobber_elf" "$crt0_obj" "$call_clobber_obj"
test -s "$call_clobber_elf"
printf 'real LLVM LNP64 lld call-clobber link smoke passed: %s\n' \
  "$call_clobber_elf"

compare_elf="$build_dir/lnp64-compare-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$compare_elf" "$crt0_obj" "$compare_obj"
test -s "$compare_elf"
printf 'real LLVM LNP64 lld comparison link smoke passed: %s\n' \
  "$compare_elf"

unsigned_compare_elf="$build_dir/lnp64-unsigned-compare-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$unsigned_compare_elf" "$crt0_obj" "$unsigned_compare_obj"
test -s "$unsigned_compare_elf"
printf 'real LLVM LNP64 lld unsigned comparison link smoke passed: %s\n' \
  "$unsigned_compare_elf"

signed_load_elf="$build_dir/lnp64-signed-load-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$signed_load_elf" "$crt0_obj" "$signed_load_obj"
test -s "$signed_load_elf"
printf 'real LLVM LNP64 lld signed-load link smoke passed: %s\n' \
  "$signed_load_elf"

wide_const_elf="$build_dir/lnp64-wide-const-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$wide_const_elf" "$crt0_obj" "$wide_const_obj"
test -s "$wide_const_elf"
printf 'real LLVM LNP64 lld wide-constant link smoke passed: %s\n' \
  "$wide_const_elf"

stack_aggregate_elf="$build_dir/lnp64-stack-aggregate-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$stack_aggregate_elf" "$crt0_obj" "$stack_aggregate_obj"
test -s "$stack_aggregate_elf"
printf 'real LLVM LNP64 lld stack aggregate link smoke passed: %s\n' \
  "$stack_aggregate_elf"

libc_string_elf="$build_dir/lnp64-libc-string-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_string_elf" "$crt0_obj" "$libc_string_obj" "$minilibc_obj"
test -s "$libc_string_elf"
printf 'real LLVM LNP64 lld minilibc string link smoke passed: %s\n' \
  "$libc_string_elf"

calloc_elf="$build_dir/lnp64-calloc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$calloc_elf" "$crt0_obj" "$calloc_obj" "$minilibc_obj"
test -s "$calloc_elf"
printf 'real LLVM LNP64 lld calloc link smoke passed: %s\n' \
  "$calloc_elf"

realloc_elf="$build_dir/lnp64-realloc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$realloc_elf" "$crt0_obj" "$realloc_obj" "$minilibc_obj"
test -s "$realloc_elf"
printf 'real LLVM LNP64 lld realloc link smoke passed: %s\n' \
  "$realloc_elf"

read_elf="$build_dir/lnp64-read-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$read_elf" "$crt0_obj" "$read_obj" "$minilibc_obj"
test -s "$read_elf"
printf 'real LLVM LNP64 lld read link smoke passed: %s\n' \
  "$read_elf"

indirect_call_elf="$build_dir/lnp64-indirect-call-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$indirect_call_elf" "$crt0_obj" "$indirect_call_obj"
test -s "$indirect_call_elf"
printf 'real LLVM LNP64 lld indirect call link smoke passed: %s\n' \
  "$indirect_call_elf"

for demo in hello factorial allocator fibonacci; do
  demo_obj="$build_dir/$demo-clang-smoke.o"
  demo_elf="$build_dir/lnp64-$demo-clang-linked.elf"
  "$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
    -o "$demo_elf" "$crt0_obj" "$demo_obj" "$minilibc_obj"
  test -s "$demo_elf"
done
printf 'real LLVM LNP64 lld clang demo link smoke passed: %s\n' \
  "$build_dir/lnp64-hello-clang-linked.elf"

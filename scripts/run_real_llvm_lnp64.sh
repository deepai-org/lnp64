#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tag="${LNP64_LLVM_PROJECT_TAG:-llvmorg-14.0.6}"
project_dir="${LNP64_LLVM_PROJECT_DIR:-target/llvm-project-src}"
build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
default_jobs="$(nproc 2>/dev/null || printf '2')"
if [[ "$default_jobs" -gt 16 ]]; then
  default_jobs=16
fi
jobs="${LNP64_LLVM_JOBS:-$default_jobs}"
gate="${LNP64_LLVM_GATE:-full}"

case "$gate" in
  full|mc|objects) ;;
  *)
    printf 'unknown LNP64_LLVM_GATE: %s\n' "$gate" >&2
    printf 'expected one of: full, mc, objects\n' >&2
    exit 2
    ;;
esac

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

case "$gate" in
  mc)
    ninja -C "$build_dir" -j "$jobs" llvm-mc llvm-objdump
    ;;
  objects)
    ninja -C "$build_dir" -j "$jobs" llc llvm-mc llvm-objdump clang
    ;;
  full)
    ninja -C "$build_dir" -j "$jobs" llc llvm-mc llvm-objdump clang lld
    ;;
esac

llvm_mc="$build_dir/bin/llvm-mc"
llvm_objdump="$build_dir/bin/llvm-objdump"

if [[ "$gate" == "mc" ]]; then
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

  auipc_asm="$build_dir/auipc-mc-smoke.s"
  cat >"$auipc_asm" <<'ASM'
  .text
  .globl _start
_start:
  auipc r1, 4096
  auipc r2, target
  ret
target:
  nop
ASM
  auipc_mc_obj="$build_dir/auipc-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$auipc_asm" \
    -o "$auipc_mc_obj"
  test -s "$auipc_mc_obj"
  auipc_mc_dump="$build_dir/auipc-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$auipc_mc_obj" \
    >"$auipc_mc_dump"
  grep -q 'auipc r1, 4096' "$auipc_mc_dump"
  grep -q 'auipc r2' "$auipc_mc_dump"
  printf 'real LLVM LNP64 llvm-mc auipc smoke passed: %s\n' "$auipc_mc_obj"

  mmap_asm="$build_dir/mmap-mc-smoke.s"
  cat >"$mmap_asm" <<'ASM'
  .text
  .globl _start
_start:
  mmap r1, r2, r3, r4
  munmap r5, r6
  mprotect r7, r8, r9, r10
  ret
ASM
  mmap_mc_obj="$build_dir/mmap-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$mmap_asm" \
    -o "$mmap_mc_obj"
  test -s "$mmap_mc_obj"
  mmap_mc_dump="$build_dir/mmap-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$mmap_mc_obj" \
    >"$mmap_mc_dump"
  grep -q 'mmap r1, r2, r3, r4' "$mmap_mc_dump"
  grep -q 'munmap r5, r6' "$mmap_mc_dump"
  grep -q 'mprotect r7, r8, r9, r10' "$mmap_mc_dump"
  printf 'real LLVM LNP64 llvm-mc mmap opcode smoke passed: %s\n' \
    "$mmap_mc_obj"

  env_get_asm="$build_dir/env-get-mc-smoke.s"
  cat >"$env_get_asm" <<'ASM'
  .text
  .globl _start
_start:
  env_get r1, r2, r3, r4
  ret
ASM
  env_get_mc_obj="$build_dir/env-get-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$env_get_asm" \
    -o "$env_get_mc_obj"
  test -s "$env_get_mc_obj"
  env_get_mc_dump="$build_dir/env-get-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$env_get_mc_obj" \
    >"$env_get_mc_dump"
  grep -q 'env_get r1, r2, r3, r4' "$env_get_mc_dump"
  printf 'real LLVM LNP64 llvm-mc env_get opcode smoke passed: %s\n' \
    "$env_get_mc_obj"

  get_pcr_asm="$build_dir/get-pcr-mc-smoke.s"
  cat >"$get_pcr_asm" <<'ASM'
  .text
  .globl _start
_start:
  get_pcr r1, PID
  set_pcr r3, SIGMASK, r2
  get_pcr r4, CRED_PROFILE
  set_pcr r5, CRED_HANDLE, r2
  ret
ASM
  get_pcr_mc_obj="$build_dir/get-pcr-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$get_pcr_asm" \
    -o "$get_pcr_mc_obj"
  test -s "$get_pcr_mc_obj"
  get_pcr_mc_dump="$build_dir/get-pcr-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$get_pcr_mc_obj" \
    >"$get_pcr_mc_dump"
  grep -q 'get_pcr r1, PID' "$get_pcr_mc_dump"
  grep -q 'set_pcr r3, SIGMASK, r2' "$get_pcr_mc_dump"
  grep -q 'get_pcr r4, CRED_PROFILE' "$get_pcr_mc_dump"
  grep -q 'set_pcr r5, CRED_HANDLE, r2' "$get_pcr_mc_dump"
  stale_set_pcr_asm="$build_dir/stale-set-pcr-mc-smoke.s"
  cat >"$stale_set_pcr_asm" <<'ASM'
  .text
  .globl _start
_start:
  set_pcr TP, r2
  ret
ASM
  stale_set_pcr_err="$build_dir/stale-set-pcr-mc-smoke.err"
  if "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$stale_set_pcr_asm" \
    -o "$build_dir/stale-set-pcr-mc-smoke.o" 2>"$stale_set_pcr_err"; then
    printf 'stale two-operand SET_PCR unexpectedly assembled\n' >&2
    exit 1
  fi
  printf 'real LLVM LNP64 llvm-mc GET_PCR opcode smoke passed: %s\n' \
    "$get_pcr_mc_obj"

  open_at_asm="$build_dir/open-at-mc-smoke.s"
  cat >"$open_at_asm" <<'ASM'
  .text
  .globl _start
_start:
  open_at r1, r2, r3, r4
ASM
  open_at_mc_obj="$build_dir/open-at-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$open_at_asm" \
    -o "$open_at_mc_obj"
  test -s "$open_at_mc_obj"
  open_at_mc_dump="$build_dir/open-at-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$open_at_mc_obj" \
    >"$open_at_mc_dump"
  grep -q 'open_at r1, r2, r3, r4' "$open_at_mc_dump"
  printf 'real LLVM LNP64 llvm-mc OPEN_AT opcode smoke passed: %s\n' \
    "$open_at_mc_obj"

  clone_control_asm="$build_dir/clone-control-mc-smoke.s"
  cat >"$clone_control_asm" <<'ASM'
  .text
  .globl _start
_start:
  clone.spawn r1, r2, r3
  thread_join r4, r5, r6
ASM
  clone_control_mc_obj="$build_dir/clone-control-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$clone_control_asm" \
    -o "$clone_control_mc_obj"
  test -s "$clone_control_mc_obj"
  clone_control_mc_dump="$build_dir/clone-control-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$clone_control_mc_obj" \
    >"$clone_control_mc_dump"
  grep -q 'clone.spawn r1, r2, r3' "$clone_control_mc_dump"
  grep -q 'thread_join r4, r5, r6' "$clone_control_mc_dump"
  printf 'real LLVM LNP64 llvm-mc clone control opcode smoke passed: %s\n' \
    "$clone_control_mc_obj"

  compat_meta_asm="$build_dir/compat-meta-mc-smoke.s"
  cat >"$compat_meta_asm" <<'ASM'
  .text
  .globl _start
_start:
  stat_path_at r1, r2, r3, r4
  stat_fd_dyn r5, r6
  utime_path_at r7, r8, r9, r10
  utime_fd_dyn r11, r12
  fcntl_fd_dyn r13, r14, r15
  fd_seek_dyn r16, r17, r18
  unlink_path_at r19, r20, r21
  open_dir_dyn r22, r23, r24
  mkdir_path_at r25, r26, r27
  rename_path_at r1, r2, r3, r4
  link_path_at r5, r6, r7, r8, r9
  symlink_path_at r10, r11, r12
  readlink_path_at r13, r14, r15, r16
  chdir_path r17
  getcwd_path r18, r19
  chmod_path_at r20, r21, r22, r23
  chown_path_at r24, r25, r26, r27, r28
  ret
ASM
  compat_meta_mc_obj="$build_dir/compat-meta-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$compat_meta_asm" \
    -o "$compat_meta_mc_obj"
  test -s "$compat_meta_mc_obj"
  compat_meta_mc_dump="$build_dir/compat-meta-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$compat_meta_mc_obj" \
    >"$compat_meta_mc_dump"
  grep -q 'stat_path_at r1, r2, r3, r4' "$compat_meta_mc_dump"
  grep -q 'stat_fd_dyn r5, r6' "$compat_meta_mc_dump"
  grep -q 'utime_path_at r7, r8, r9, r10' "$compat_meta_mc_dump"
  grep -q 'utime_fd_dyn r11, r12' "$compat_meta_mc_dump"
  grep -q 'fcntl_fd_dyn r13, r14, r15' "$compat_meta_mc_dump"
  grep -q 'fd_seek_dyn r16, r17, r18' "$compat_meta_mc_dump"
  grep -q 'unlink_path_at r19, r20, r21' "$compat_meta_mc_dump"
  grep -q 'open_dir_dyn r22, r23, r24' "$compat_meta_mc_dump"
  grep -q 'mkdir_path_at r25, r26, r27' "$compat_meta_mc_dump"
  grep -q 'rename_path_at r1, r2, r3, r4' "$compat_meta_mc_dump"
  grep -q 'link_path_at r5, r6, r7, r8, r9' "$compat_meta_mc_dump"
  grep -q 'symlink_path_at r10, r11, r12' "$compat_meta_mc_dump"
  grep -q 'readlink_path_at r13, r14, r15, r16' "$compat_meta_mc_dump"
  grep -q 'chdir_path r17' "$compat_meta_mc_dump"
  grep -q 'getcwd_path r18, r19' "$compat_meta_mc_dump"
  grep -q 'chmod_path_at r20, r21, r22, r23' "$compat_meta_mc_dump"
  grep -q 'chown_path_at r24, r25, r26, r27, r28' "$compat_meta_mc_dump"
  printf 'real LLVM LNP64 llvm-mc compatibility metadata opcode smoke passed: %s\n' \
    "$compat_meta_mc_obj"

  cap_control_asm="$build_dir/cap-control-mc-smoke.s"
  cat >"$cap_control_asm" <<'ASM'
  .text
  .globl _start
_start:
  cap_dup r1, r2
  cap_send r3, r4
  cap_recv r5, r6
  cap_revoke r7, r8
  ret
ASM
  cap_control_mc_obj="$build_dir/cap-control-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$cap_control_asm" \
    -o "$cap_control_mc_obj"
  test -s "$cap_control_mc_obj"
  cap_control_mc_dump="$build_dir/cap-control-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$cap_control_mc_obj" \
    >"$cap_control_mc_dump"
  grep -q 'cap_dup r1, r2' "$cap_control_mc_dump"
  grep -q 'cap_send r3, r4' "$cap_control_mc_dump"
  grep -q 'cap_recv r5, r6' "$cap_control_mc_dump"
  grep -q 'cap_revoke r7, r8' "$cap_control_mc_dump"
  printf 'real LLVM LNP64 llvm-mc capability control opcode smoke passed: %s\n' \
    "$cap_control_mc_obj"

  atomic_asm="$build_dir/atomic-mc-smoke.s"
  cat >"$atomic_asm" <<'ASM'
  .text
  .globl _start
_start:
  amo.swap r1, r2, r3
  amo.add r4, r5, r6
  amo.and r7, r8, r9
  amo.or r10, r11, r12
  lock.cmpxchg r13, r14, r15, r16
  amo.xor r17, r18, r19
  futex_wait r20, r21
  futex_wake r22, r23
  fence
  fence.acq
  fence.rel
  fence.acq_rel
  fence.sc
  isync r24, r25, r26
  ret
ASM
  atomic_mc_obj="$build_dir/atomic-mc-smoke.o"
  "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$atomic_asm" \
    -o "$atomic_mc_obj"
  test -s "$atomic_mc_obj"
  atomic_mc_dump="$build_dir/atomic-mc-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$atomic_mc_obj" \
    >"$atomic_mc_dump"
  grep -q 'amo.swap r1, r2, r3' "$atomic_mc_dump"
  grep -q 'amo.add r4, r5, r6' "$atomic_mc_dump"
  grep -q 'amo.and r7, r8, r9' "$atomic_mc_dump"
  grep -q 'amo.or r10, r11, r12' "$atomic_mc_dump"
  grep -q 'lock.cmpxchg r13, r14, r15, r16' "$atomic_mc_dump"
  grep -q 'amo.xor r17, r18, r19' "$atomic_mc_dump"
  grep -q 'futex_wait r20, r21' "$atomic_mc_dump"
  grep -q 'futex_wake r22, r23' "$atomic_mc_dump"
  grep -q 'fence' "$atomic_mc_dump"
  grep -q 'isync r24, r25, r26' "$atomic_mc_dump"
  printf 'real LLVM LNP64 llvm-mc atomic opcode smoke passed: %s\n' \
    "$atomic_mc_obj"

  minilibc_dump="$build_dir/liblnp64-min-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$minilibc_obj" \
    >"$minilibc_dump"
  grep -q 'pull r1, r1, r2, r3' "$minilibc_dump"
  grep -q 'alloc r1, r1' "$minilibc_dump"
  grep -q 'alloc_size r3, r2' "$minilibc_dump"
  grep -q 'free r1' "$minilibc_dump"
  printf 'real LLVM LNP64 llvm-objdump minilibc native decode smoke passed: %s\n' \
    "$minilibc_dump"

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
  exit 0
fi

llc="$build_dir/bin/llc"
clang="$build_dir/bin/clang"
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -O1 -ffreestanding -fno-pic -fno-jump-tables \
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

debug_line_c="$build_dir/debug-line-smoke.c"
cat >"$debug_line_c" <<'C'
__attribute__((noinline)) int debug_line_probe(int x) {
  int y = x + 7;
  return y * 3;
}
C

debug_line_obj="$build_dir/debug-line-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O0 -g -gdwarf-5 -ffreestanding \
  -fno-pic -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I toolchain -c "$debug_line_c" -o "$debug_line_obj"
test -s "$debug_line_obj"
debug_line_sections="$build_dir/debug-line-clang-smoke.sections"
"$llvm_objdump" -h --triple=lnp64-unknown-none "$debug_line_obj" \
  >"$debug_line_sections"
grep -q '.debug_info' "$debug_line_sections"
grep -q '.debug_line' "$debug_line_sections"
grep -q '.debug_frame' "$debug_line_sections"
grep -q '.rela.debug_line' "$debug_line_sections"
printf 'real LLVM LNP64 clang debug section smoke passed: %s\n' \
  "$debug_line_obj"

hello_obj="$build_dir/hello-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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

pcr_obj="$build_dir/pcr-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/pcr.c -o "$pcr_obj"
test -s "$pcr_obj"
pcr_dump="$build_dir/pcr-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$pcr_obj" >"$pcr_dump"
grep -q 'call ' "$pcr_dump"
printf 'real LLVM LNP64 clang PCR demo object smoke passed: %s\n' \
  "$pcr_obj"

cat_obj="$build_dir/cat-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/cat.c -o "$cat_obj"
test -s "$cat_obj"
cat_dump="$build_dir/cat-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$cat_obj" >"$cat_dump"
grep -q 'call ' "$cat_dump"
printf 'real LLVM LNP64 clang cat demo object smoke passed: %s\n' \
  "$cat_obj"

json_parser_obj="$build_dir/json-parser-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/json_parser.c -o "$json_parser_obj"
test -s "$json_parser_obj"
json_parser_dump="$build_dir/json-parser-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$json_parser_obj" \
  >"$json_parser_dump"
grep -q 'ld r' "$json_parser_dump"
grep -q 'st r' "$json_parser_dump"
grep -q 'call ' "$json_parser_dump"
printf 'real LLVM LNP64 clang json parser demo object smoke passed: %s\n' \
  "$json_parser_obj"

rot13_obj="$build_dir/rot13-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/rot13.c -o "$rot13_obj"
test -s "$rot13_obj"
rot13_dump="$build_dir/rot13-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$rot13_obj" >"$rot13_dump"
grep -q 'ld.b r' "$rot13_dump"
grep -q 'st.b r' "$rot13_dump"
grep -q 'call ' "$rot13_dump"
printf 'real LLVM LNP64 clang rot13 demo object smoke passed: %s\n' \
  "$rot13_obj"

producer_consumer_obj="$build_dir/producer-consumer-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/producer_consumer.c -o "$producer_consumer_obj"
test -s "$producer_consumer_obj"
producer_consumer_dump="$build_dir/producer-consumer-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$producer_consumer_obj" \
  >"$producer_consumer_dump"
grep -q 'clone.spawn r' "$producer_consumer_dump"
grep -q 'thread_join r' "$producer_consumer_dump"
grep -q 'lock.cmpxchg r' "$producer_consumer_dump"
printf 'real LLVM LNP64 clang producer consumer demo object smoke passed: %s\n' \
  "$producer_consumer_obj"

parallel_hash_obj="$build_dir/parallel-hash-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/parallel_hash.c -o "$parallel_hash_obj"
test -s "$parallel_hash_obj"
parallel_hash_dump="$build_dir/parallel-hash-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$parallel_hash_obj" \
  >"$parallel_hash_dump"
grep -q 'clone.spawn r' "$parallel_hash_dump"
grep -q 'thread_join r' "$parallel_hash_dump"
grep -q 'amo.add r' "$parallel_hash_dump"
printf 'real LLVM LNP64 clang parallel hash demo object smoke passed: %s\n' \
  "$parallel_hash_obj"

sqlite_lite_obj="$build_dir/sqlite-lite-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/sqlite_lite.c -o "$sqlite_lite_obj"
test -s "$sqlite_lite_obj"
sqlite_lite_dump="$build_dir/sqlite-lite-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$sqlite_lite_obj" \
  >"$sqlite_lite_dump"
grep -q 'mmap r' "$sqlite_lite_dump"
grep -q 'clone.spawn r' "$sqlite_lite_dump"
grep -q 'thread_join r' "$sqlite_lite_dump"
grep -q 'lock.cmpxchg r' "$sqlite_lite_dump"
grep -q 'fence' "$sqlite_lite_dump"
printf 'real LLVM LNP64 clang sqlite lite demo object smoke passed: %s\n' \
  "$sqlite_lite_obj"

ping_pong_obj="$build_dir/ping-pong-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/ping_pong.c -o "$ping_pong_obj"
test -s "$ping_pong_obj"
ping_pong_dump="$build_dir/ping-pong-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$ping_pong_obj" \
  >"$ping_pong_dump"
grep -q 'object_ctl r' "$ping_pong_dump"
grep -q 'push r' "$ping_pong_dump"
grep -q 'pull r' "$ping_pong_dump"
grep -q 'clone.spawn r' "$ping_pong_dump"
grep -q 'thread_join r' "$ping_pong_dump"
printf 'real LLVM LNP64 clang ping pong demo object smoke passed: %s\n' \
  "$ping_pong_obj"

zlib_adler_obj="$build_dir/zlib-adler32-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O0 -ffreestanding -fno-builtin \
  -fno-pic -fno-jump-tables -fno-unwind-tables \
  -fno-asynchronous-unwind-tables -I toolchain/include -I third_party/zlib \
  -c third_party/zlib/adler32.c -o "$zlib_adler_obj"
test -s "$zlib_adler_obj"
zlib_adler_dump="$build_dir/zlib-adler32-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$zlib_adler_obj" \
  >"$zlib_adler_dump"
grep -q '<adler32_z>:' "$zlib_adler_dump"
printf 'real LLVM LNP64 clang zlib adler32 object smoke passed: %s\n' \
  "$zlib_adler_obj"

zlib_crc_obj="$build_dir/zlib-crc32-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O0 -ffreestanding -fno-builtin \
  -fno-pic -fno-jump-tables -fno-unwind-tables \
  -fno-asynchronous-unwind-tables -I toolchain/include -I third_party/zlib \
  -c third_party/zlib/crc32.c -o "$zlib_crc_obj"
test -s "$zlib_crc_obj"
zlib_crc_dump="$build_dir/zlib-crc32-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$zlib_crc_obj" \
  >"$zlib_crc_dump"
grep -q '<crc32_z>:' "$zlib_crc_dump"
printf 'real LLVM LNP64 clang zlib crc32 object smoke passed: %s\n' \
  "$zlib_crc_obj"

zlib_smoke_c="$build_dir/zlib-smoke.c"
cat >"$zlib_smoke_c" <<'C'
#include "zlib.h"

int main(void) {
  const Bytef data[] = "hello zlib";
  uLong adler = adler32(0L, Z_NULL, 0);
  uLong crc = crc32(0L, Z_NULL, 0);
  adler = adler32(adler, data, 10);
  crc = crc32(crc, data, 10);
  if (adler != 0x159503e6UL)
    return 1;
  if (crc != 0x96b34bd1UL)
    return 2;
  return 0;
}
C

zlib_smoke_obj="$build_dir/zlib-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O0 -ffreestanding -fno-builtin \
  -fno-pic -fno-jump-tables -fno-unwind-tables \
  -fno-asynchronous-unwind-tables -I toolchain/include -I third_party/zlib \
  -c "$zlib_smoke_c" -o "$zlib_smoke_obj"
test -s "$zlib_smoke_obj"
zlib_smoke_dump="$build_dir/zlib-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$zlib_smoke_obj" \
  >"$zlib_smoke_dump"
grep -q '<main>:' "$zlib_smoke_dump"
grep -q 'call ' "$zlib_smoke_dump"
printf 'real LLVM LNP64 clang zlib package object smoke passed: %s\n' \
  "$zlib_smoke_obj"

natsort_impl_obj="$build_dir/natsort-strnatcmp-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I toolchain/include -I third_party/natsort \
  -c third_party/natsort/strnatcmp.c -o "$natsort_impl_obj"
test -s "$natsort_impl_obj"
natsort_impl_dump="$build_dir/natsort-strnatcmp-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$natsort_impl_obj" \
  >"$natsort_impl_dump"
grep -q '<strnatcmp>:' "$natsort_impl_dump"
grep -q '<strnatcasecmp>:' "$natsort_impl_dump"
grep -q 'call ' "$natsort_impl_dump"
printf 'real LLVM LNP64 clang natsort implementation object smoke passed: %s\n' \
  "$natsort_impl_obj"

natsort_smoke_c="$build_dir/natsort-smoke.c"
cat >"$natsort_smoke_c" <<'C'
#include "strnatcmp.h"

int main(void) {
  if (!(strnatcmp("rfc822.txt", "rfc2086.txt") < 0))
    return 1;
  if (!(strnatcmp("a10", "a2") > 0))
    return 2;
  if (strnatcmp("x2-y08", "x2-y7") >= 0)
    return 3;
  if (strnatcasecmp("File9", "file10") >= 0)
    return 4;
  return 0;
}
C

natsort_smoke_obj="$build_dir/natsort-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I toolchain/include -I third_party/natsort \
  -c "$natsort_smoke_c" -o "$natsort_smoke_obj"
test -s "$natsort_smoke_obj"
natsort_smoke_dump="$build_dir/natsort-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$natsort_smoke_obj" \
  >"$natsort_smoke_dump"
grep -q 'call ' "$natsort_smoke_dump"
printf 'real LLVM LNP64 clang natsort package object smoke passed: %s\n' \
  "$natsort_smoke_obj"

jsmn_smoke_c="$build_dir/jsmn-smoke.c"
cat >"$jsmn_smoke_c" <<'C'
#include "jsmn.h"

static unsigned long lnp64_len(const char *s) {
  unsigned long n = 0;
  while (s[n])
    n = n + 1;
  return n;
}

int main(void) {
  const char *json = "{\"name\":\"lnp64\",\"ok\":true}";
  jsmn_parser parser;
  jsmntok_t tok[8];
  int r;

  jsmn_init(&parser);
  r = jsmn_parse(&parser, json, lnp64_len(json), tok, 8);
  if (r != 5)
    return 1;
  if (tok[0].type != JSMN_OBJECT || tok[0].size != 2)
    return 2;
  if (tok[1].type != JSMN_STRING || tok[3].type != JSMN_STRING)
    return 3;
  if (tok[4].type != JSMN_PRIMITIVE)
    return 4;
  return 0;
}
C

jsmn_smoke_obj="$build_dir/jsmn-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I toolchain/include -I third_party/jsmn \
  -c "$jsmn_smoke_c" -o "$jsmn_smoke_obj"
test -s "$jsmn_smoke_obj"
jsmn_smoke_dump="$build_dir/jsmn-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$jsmn_smoke_obj" \
  >"$jsmn_smoke_dump"
grep -q '<jsmn_parse>:' "$jsmn_smoke_dump"
grep -q '<main>:' "$jsmn_smoke_dump"
printf 'real LLVM LNP64 clang jsmn package object smoke passed: %s\n' \
  "$jsmn_smoke_obj"

inih_smoke_c="$build_dir/inih-smoke.c"
cat >"$inih_smoke_c" <<'C'
#define INI_API static
#include "ini.c"

static int seen;

static int streq(const char *a, const char *b) {
  while (*a && *b && *a == *b) {
    a = a + 1;
    b = b + 1;
  }
  return *a == *b;
}

static int handler(void *user, const char *section, const char *name,
                   const char *value) {
  (void)user;
  if (streq(section, "server") && streq(name, "host") &&
      streq(value, "localhost"))
    seen = seen + 1;
  if (streq(section, "server") && streq(name, "port") &&
      streq(value, "41066"))
    seen = seen + 2;
  if (streq(section, "feature") && streq(name, "enabled") &&
      streq(value, "yes"))
    seen = seen + 4;
  return 1;
}

int main(void) {
  int rc;
  seen = 0;
  rc = ini_parse_string("[server]\nhost=localhost\nport=41066\n[feature]\nenabled=yes\n",
                        handler, 0);
  if (rc != 0)
    return 1;
  if (seen != 7)
    return 2;
  return 0;
}
C

inih_smoke_obj="$build_dir/inih-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O0 -ffreestanding -fno-builtin -fno-pic \
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -I toolchain/include -I third_party/inih \
  -c "$inih_smoke_c" -o "$inih_smoke_obj"
test -s "$inih_smoke_obj"
inih_smoke_dump="$build_dir/inih-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$inih_smoke_obj" \
  >"$inih_smoke_dump"
grep -q '<ini_parse_string>:' "$inih_smoke_dump"
grep -q '<main>:' "$inih_smoke_dump"
printf 'real LLVM LNP64 clang inih package object smoke passed: %s\n' \
  "$inih_smoke_obj"

cwalk_impl_obj="$build_dir/cwalk-clang-impl.o"
"$clang" --target=lnp64-unknown-none -O0 -DNDEBUG -ffreestanding \
  -fno-builtin -fno-pic -fno-jump-tables -fno-unwind-tables \
  -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/cwalk/include \
  -c third_party/cwalk/src/cwalk.c -o "$cwalk_impl_obj"
test -s "$cwalk_impl_obj"
cwalk_impl_dump="$build_dir/cwalk-clang-impl.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$cwalk_impl_obj" \
  >"$cwalk_impl_dump"
grep -q '<cwk_path_normalize>:' "$cwalk_impl_dump"
grep -q '<cwk_path_join>:' "$cwalk_impl_dump"
printf 'real LLVM LNP64 clang cwalk implementation object smoke passed: %s\n' \
  "$cwalk_impl_obj"

cwalk_smoke_c="$build_dir/cwalk-smoke.c"
cat >"$cwalk_smoke_c" <<'C'
#include "cwalk.h"

static int streq(const char *a, const char *b) {
  while (*a && *b && *a == *b) {
    a = a + 1;
    b = b + 1;
  }
  return *a == *b;
}

int main(void) {
  char buffer[128];
  const char *base;
  size_t length;
  size_t written;
  struct cwk_segment segment;

  cwk_path_set_style(CWK_STYLE_UNIX);
  if (!cwk_path_is_absolute("/tmp/archive.tar.gz"))
    return 1;
  if (!cwk_path_is_relative("tmp/archive.tar.gz"))
    return 2;
  cwk_path_get_basename("/tmp/archive.tar.gz", &base, &length);
  if (length != 14)
    return 3;
  if (!cwk_path_get_first_segment("/tmp/archive.tar.gz", &segment))
    return 4;
  if (segment.size != 3)
    return 5;
  written = cwk_path_normalize("/var//log/../tmp/./cache/", buffer,
                               sizeof(buffer));
  if (written != 14 || !streq(buffer, "/var/tmp/cache"))
    return 6;
  written = cwk_path_join("/var/log", "../tmp/app.log", buffer,
                          sizeof(buffer));
  if (written != 16 || !streq(buffer, "/var/tmp/app.log"))
    return 7;
  cwk_path_set_style(CWK_STYLE_WINDOWS);
  written = cwk_path_normalize("C:/temp\\..//out\\file.txt", buffer,
                               sizeof(buffer));
  if (written != 15 || !streq(buffer, "C:\\out\\file.txt"))
    return 8;
  return 0;
}
C

cwalk_smoke_obj="$build_dir/cwalk-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O0 -ffreestanding -fno-builtin \
  -fno-pic -fno-jump-tables -fno-unwind-tables \
  -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/cwalk/include \
  -c "$cwalk_smoke_c" -o "$cwalk_smoke_obj"
test -s "$cwalk_smoke_obj"
cwalk_smoke_dump="$build_dir/cwalk-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$cwalk_smoke_obj" \
  >"$cwalk_smoke_dump"
grep -q '<main>:' "$cwalk_smoke_dump"
grep -q 'call ' "$cwalk_smoke_dump"
printf 'real LLVM LNP64 clang cwalk package object smoke passed: %s\n' \
  "$cwalk_smoke_obj"

varargs_call_c="$build_dir/varargs-call-smoke.c"
cat >"$varargs_call_c" <<'C'
#include <stdio.h>

int main(void) {
  return printf("lnp64 %d %s\n", 64, "varargs");
}
C

varargs_call_obj="$build_dir/varargs-call-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -O0 -ffreestanding -fno-builtin \
  -fno-pic -fno-jump-tables -fno-unwind-tables \
  -fno-asynchronous-unwind-tables -I toolchain/include \
  -c "$varargs_call_c" -o "$varargs_call_obj"
test -s "$varargs_call_obj"
varargs_call_dump="$build_dir/varargs-call-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$varargs_call_obj" \
  >"$varargs_call_dump"
grep -q '<main>:' "$varargs_call_dump"
grep -q 'call ' "$varargs_call_dump"
printf 'real LLVM LNP64 clang varargs call object smoke passed: %s\n' \
  "$varargs_call_obj"

sbase_commands=(
  echo cat wc yes basename dirname head tee cksum tail cmp uniq sort grep sed
  cp mv ls chmod chown ln mkdir rm cut tr touch find
)
sbase_objs=()
for sbase_cmd in "${sbase_commands[@]}"; do
  sbase_obj="$build_dir/sbase-$sbase_cmd-clang-smoke.o"
  "$clang" --target=lnp64-unknown-none -O0 \
    -Werror=implicit-function-declaration -ffreestanding -fno-builtin \
    -fno-pic -fno-jump-tables -fno-unwind-tables \
    -fno-asynchronous-unwind-tables -I toolchain/include -I third_party/sbase \
    -c "third_party/sbase/$sbase_cmd.c" -o "$sbase_obj"
  test -s "$sbase_obj"
  sbase_dump="$build_dir/sbase-$sbase_cmd-clang-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$sbase_obj" \
    >"$sbase_dump"
  grep -q '<main>:' "$sbase_dump"
  sbase_objs+=("$sbase_obj")
done
printf 'real LLVM LNP64 clang sbase command object smokes passed: %s\n' \
  "${sbase_objs[*]}"

sbase_libutil_sources=(
  concat confirm cp enmasse fnck getlines linecmp writeall
)
sbase_libutil_objs=()
for sbase_libutil in "${sbase_libutil_sources[@]}"; do
  sbase_libutil_obj="$build_dir/sbase-libutil-$sbase_libutil-clang-smoke.o"
  "$clang" --target=lnp64-unknown-none -O0 \
    -Werror=implicit-function-declaration -ffreestanding -fno-builtin \
    -fno-pic -fno-jump-tables -fno-unwind-tables \
    -fno-asynchronous-unwind-tables -I toolchain/include -I third_party/sbase \
    -c "third_party/sbase/libutil/$sbase_libutil.c" \
    -o "$sbase_libutil_obj"
  test -s "$sbase_libutil_obj"
  sbase_libutil_dump="$build_dir/sbase-libutil-$sbase_libutil-clang-smoke.dump"
  "$llvm_objdump" -d --triple=lnp64-unknown-none "$sbase_libutil_obj" \
    >"$sbase_libutil_dump"
  grep -q "<$sbase_libutil>:" "$sbase_libutil_dump"
  sbase_libutil_objs+=("$sbase_libutil_obj")
done
printf 'real LLVM LNP64 clang sbase libutil object smokes passed: %s\n' \
  "${sbase_libutil_objs[*]}"

sbase_support_impl_c="toolchain/liblnp64_sbase_min.c"
sbase_support_impl_obj="$build_dir/liblnp64-sbase-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin \
  -fno-pic -fno-jump-tables -fno-unwind-tables \
  -fno-asynchronous-unwind-tables -I toolchain/include -I third_party/sbase \
  -c "$sbase_support_impl_c" -o "$sbase_support_impl_obj"
test -s "$sbase_support_impl_obj"
sbase_support_impl_dump="$build_dir/liblnp64-sbase-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$sbase_support_impl_obj" \
  >"$sbase_support_impl_dump"
grep -q '<putword>:' "$sbase_support_impl_dump"
grep -q '<putchar>:' "$sbase_support_impl_dump"
grep -q '<eprintf>:' "$sbase_support_impl_dump"
printf 'real LLVM LNP64 clang sbase support implementation object smoke passed: %s\n' \
  "$sbase_support_impl_obj"

netcat_obj="$build_dir/netcat-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/netcat.c -o "$netcat_obj"
test -s "$netcat_obj"
netcat_dump="$build_dir/netcat-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netcat_obj" \
  >"$netcat_dump"
grep -q 'call ' "$netcat_dump"
printf 'real LLVM LNP64 clang netcat demo object smoke passed: %s\n' \
  "$netcat_obj"

httpd_obj="$build_dir/httpd-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c demos/httpd.c -o "$httpd_obj"
test -s "$httpd_obj"
httpd_dump="$build_dir/httpd-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$httpd_obj" \
  >"$httpd_dump"
grep -q 'call ' "$httpd_dump"
printf 'real LLVM LNP64 clang httpd demo object smoke passed: %s\n' \
  "$httpd_obj"

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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
  if (__lnp_object_create(999, 0, 0, 0, 0) != (lnp64_word_t)-1)
    return 3;
  return 0;
}
C

intrinsic_ctl_obj="$build_dir/intrinsic-control-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
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

intrinsic_cap_c="$build_dir/intrinsic-cap-control.c"
cat >"$intrinsic_cap_c" <<'C'
#include "lnp64_intrinsics.h"

int main(void) {
  if (__lnp_cap_dup(999, 0, 0) != (lnp64_word_t)-1)
    return 1;
  if (__lnp_cap_send(998, 997, 0) != (lnp64_word_t)-1)
    return 2;
  if (__lnp_cap_recv(996, 0) != (lnp64_word_t)-1)
    return 3;
  if (__lnp_cap_revoke(995, 0) != (lnp64_word_t)-1)
    return 4;
  return 0;
}
C

intrinsic_cap_obj="$build_dir/intrinsic-cap-control-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_cap_c" -o "$intrinsic_cap_obj"
test -s "$intrinsic_cap_obj"
intrinsic_cap_dump="$build_dir/intrinsic-cap-control-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_cap_obj" \
  >"$intrinsic_cap_dump"
grep -q 'cap_dup r' "$intrinsic_cap_dump"
grep -q 'cap_send r' "$intrinsic_cap_dump"
grep -q 'cap_recv r' "$intrinsic_cap_dump"
grep -q 'cap_revoke r' "$intrinsic_cap_dump"
printf 'real LLVM LNP64 clang intrinsic capability control object smoke passed: %s\n' \
  "$intrinsic_cap_obj"

intrinsic_mmap_c="$build_dir/intrinsic-mmap.c"
cat >"$intrinsic_mmap_c" <<'C'
#include "lnp64_intrinsics.h"

static volatile void *mapped;

int main(void) {
  void *p = __lnp_mmap_bootstrap(0, 4096, 3);
  mapped = p;
  if (__lnp_mprotect_bootstrap(p, 4096, 1) != 0)
    return 1;
  return (int)__lnp_munmap_bootstrap(p);
}
C

intrinsic_mmap_obj="$build_dir/intrinsic-mmap-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_mmap_c" -o "$intrinsic_mmap_obj"
test -s "$intrinsic_mmap_obj"
intrinsic_mmap_dump="$build_dir/intrinsic-mmap-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_mmap_obj" \
  >"$intrinsic_mmap_dump"
grep -q 'mmap r' "$intrinsic_mmap_dump"
grep -q 'mprotect r' "$intrinsic_mmap_dump"
grep -q 'munmap r' "$intrinsic_mmap_dump"
printf 'real LLVM LNP64 clang intrinsic mmap object smoke passed: %s\n' \
  "$intrinsic_mmap_obj"

intrinsic_get_pcr_c="$build_dir/intrinsic-get-pcr.c"
cat >"$intrinsic_get_pcr_c" <<'C'
#include "lnp64_intrinsics.h"

int main(void) {
  return __lnp_get_pid() == 1 ? 0 : 1;
}
C

intrinsic_get_pcr_obj="$build_dir/intrinsic-get-pcr-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_get_pcr_c" -o "$intrinsic_get_pcr_obj"
test -s "$intrinsic_get_pcr_obj"
intrinsic_get_pcr_dump="$build_dir/intrinsic-get-pcr-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_get_pcr_obj" \
  >"$intrinsic_get_pcr_dump"
grep -q 'get_pcr r' "$intrinsic_get_pcr_dump"
printf 'real LLVM LNP64 clang intrinsic GET_PCR object smoke passed: %s\n' \
  "$intrinsic_get_pcr_obj"

intrinsic_set_pcr_c="$build_dir/intrinsic-set-pcr.c"
cat >"$intrinsic_set_pcr_c" <<'C'
#include "lnp64_intrinsics.h"

int main(void) {
  lnp64_word_t old_tp = __lnp_get_thread_pointer();
  if (__lnp_set_thread_pointer(0x1234) != 0)
    return 1;
  if (__lnp_get_thread_pointer() != 0x1234)
    return 2;
  if (__lnp_set_thread_pointer(old_tp) != 0)
    return 3;
  if (__lnp_set_event_mask(0x55) != 0)
    return 4;
  return __lnp_get_event_mask() == 0x55 ? 0 : 5;
}
C

intrinsic_set_pcr_obj="$build_dir/intrinsic-set-pcr-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_set_pcr_c" -o "$intrinsic_set_pcr_obj"
test -s "$intrinsic_set_pcr_obj"
intrinsic_set_pcr_dump="$build_dir/intrinsic-set-pcr-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_set_pcr_obj" \
  >"$intrinsic_set_pcr_dump"
grep -q 'get_pcr r' "$intrinsic_set_pcr_dump"
grep -q 'set_pcr r' "$intrinsic_set_pcr_dump"
printf 'real LLVM LNP64 clang intrinsic SET_PCR object smoke passed: %s\n' \
  "$intrinsic_set_pcr_obj"

intrinsic_openat_c="$build_dir/intrinsic-openat.c"
cat >"$intrinsic_openat_c" <<'C'
#include "lnp64_intrinsics.h"

int main(void) {
  lnp64_word_t fd = __lnp_openat((lnp64_cap_t)(long)-100,
                                 (lnp64_word_t)"demos/cat_input.txt", 0, 0);
  return fd == (lnp64_word_t)-1 ? 1 : 0;
}
C

intrinsic_openat_obj="$build_dir/intrinsic-openat-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_openat_c" -o "$intrinsic_openat_obj"
test -s "$intrinsic_openat_obj"
intrinsic_openat_dump="$build_dir/intrinsic-openat-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_openat_obj" \
  >"$intrinsic_openat_dump"
grep -q 'open_at r' "$intrinsic_openat_dump"
printf 'real LLVM LNP64 clang intrinsic OPEN_AT object smoke passed: %s\n' \
  "$intrinsic_openat_obj"

intrinsic_clone_c="$build_dir/intrinsic-clone.c"
cat >"$intrinsic_clone_c" <<'C'
#include "lnp64_intrinsics.h"

static volatile lnp64_word_t marker;

static int child(lnp64_word_t arg) {
  marker = arg + 1;
  __lnp_exit(0);
  return 0;
}

int main(void) {
  lnp64_word_t tid = __lnp_spawn_entry((lnp64_word_t)child, 41);
  if (tid == (lnp64_word_t)-1)
    return 1;
  if (__lnp_thread_join(tid, 0) != 0)
    return 2;
  return marker == 42 ? 0 : 3;
}
C

intrinsic_clone_obj="$build_dir/intrinsic-clone-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_clone_c" -o "$intrinsic_clone_obj"
test -s "$intrinsic_clone_obj"
intrinsic_clone_dump="$build_dir/intrinsic-clone-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_clone_obj" \
  >"$intrinsic_clone_dump"
grep -q 'clone.spawn r' "$intrinsic_clone_dump"
grep -q 'thread_join r' "$intrinsic_clone_dump"
printf 'real LLVM LNP64 clang intrinsic CLONE object smoke passed: %s\n' \
  "$intrinsic_clone_obj"

intrinsic_amo_c="$build_dir/intrinsic-amo.c"
cat >"$intrinsic_amo_c" <<'C'
#include "lnp64_intrinsics.h"

static volatile lnp64_word_t cell = 7;

int main(void) {
  if (__lnp_amo_add(&cell, 5) != 7)
    return 1;
  if (cell != 12)
    return 2;
  if (__lnp_amo_and(&cell, 10) != 12)
    return 3;
  if (cell != 8)
    return 4;
  if (__lnp_amo_or(&cell, 3) != 8)
    return 5;
  if (cell != 11)
    return 6;
  if (__lnp_amo_xor(&cell, 6) != 11)
    return 7;
  if (cell != 13)
    return 8;
  if (__lnp_amo_swap(&cell, 42) != 13)
    return 9;
  return cell == 42 ? 0 : 10;
}
C

intrinsic_amo_obj="$build_dir/intrinsic-amo-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_amo_c" -o "$intrinsic_amo_obj"
test -s "$intrinsic_amo_obj"
intrinsic_amo_dump="$build_dir/intrinsic-amo-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_amo_obj" \
  >"$intrinsic_amo_dump"
grep -q 'amo.add r' "$intrinsic_amo_dump"
grep -q 'amo.and r' "$intrinsic_amo_dump"
grep -q 'amo.or r' "$intrinsic_amo_dump"
grep -q 'amo.xor r' "$intrinsic_amo_dump"
grep -q 'amo.swap r' "$intrinsic_amo_dump"
printf 'real LLVM LNP64 clang intrinsic AMO object smoke passed: %s\n' \
  "$intrinsic_amo_obj"

intrinsic_futex_c="$build_dir/intrinsic-futex.c"
cat >"$intrinsic_futex_c" <<'C'
#include "lnp64_intrinsics.h"

static volatile lnp64_word_t futex_cell = 1;

int main(void) {
  __lnp_futex_wait(&futex_cell, 0);
  return __lnp_futex_wake(&futex_cell, 1);
}
C

intrinsic_futex_obj="$build_dir/intrinsic-futex-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$intrinsic_futex_c" -o "$intrinsic_futex_obj"
test -s "$intrinsic_futex_obj"
intrinsic_futex_dump="$build_dir/intrinsic-futex-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$intrinsic_futex_obj" \
  >"$intrinsic_futex_dump"
grep -q 'futex_wait r' "$intrinsic_futex_dump"
grep -q 'futex_wake r' "$intrinsic_futex_dump"
printf 'real LLVM LNP64 clang intrinsic futex object smoke passed: %s\n' \
  "$intrinsic_futex_obj"

c11_atomic_c="$build_dir/c11-atomic-smoke.c"
cat >"$c11_atomic_c" <<'C'
static unsigned long cell = 7;

int main(void) {
  unsigned long loaded = __atomic_load_n(&cell, __ATOMIC_SEQ_CST);
  if (loaded != 7)
    return 1;

  __atomic_store_n(&cell, 9, __ATOMIC_SEQ_CST);
  if (__atomic_load_n(&cell, __ATOMIC_SEQ_CST) != 9)
    return 2;

  unsigned long old_add = __atomic_fetch_add(&cell, 3, __ATOMIC_SEQ_CST);
  if (old_add != 9 || cell != 12)
    return 3;

  unsigned long old_and = __atomic_fetch_and(&cell, 10, __ATOMIC_SEQ_CST);
  if (old_and != 12 || cell != 8)
    return 4;

  unsigned long old_or = __atomic_fetch_or(&cell, 3, __ATOMIC_SEQ_CST);
  if (old_or != 8 || cell != 11)
    return 5;

  unsigned long old_xor = __atomic_fetch_xor(&cell, 6, __ATOMIC_SEQ_CST);
  if (old_xor != 11 || cell != 13)
    return 6;

  unsigned long old_swap = __atomic_exchange_n(&cell, 42, __ATOMIC_SEQ_CST);
  if (old_swap != 13 || cell != 42)
    return 7;

  unsigned long expected = 42;
  int exchanged = __atomic_compare_exchange_n(&cell, &expected, 99, 0,
                                              __ATOMIC_SEQ_CST,
                                              __ATOMIC_SEQ_CST);
  if (!exchanged || expected != 42 || cell != 99)
    return 8;

  expected = 42;
  exchanged = __atomic_compare_exchange_n(&cell, &expected, 123, 0,
                                          __ATOMIC_SEQ_CST,
                                          __ATOMIC_SEQ_CST);
  if (exchanged || expected != 99 || cell != 99)
    return 9;

  return 0;
}
C

c11_atomic_obj="$build_dir/c11-atomic-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$c11_atomic_c" -o "$c11_atomic_obj"
test -s "$c11_atomic_obj"
c11_atomic_dump="$build_dir/c11-atomic-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$c11_atomic_obj" \
  >"$c11_atomic_dump"
grep -q 'amo.add r' "$c11_atomic_dump"
grep -q 'amo.and r' "$c11_atomic_dump"
grep -q 'amo.or r' "$c11_atomic_dump"
grep -q 'amo.xor r' "$c11_atomic_dump"
grep -q 'amo.swap r' "$c11_atomic_dump"
grep -q 'lock.cmpxchg r' "$c11_atomic_dump"
printf 'real LLVM LNP64 clang C11 atomic object smoke passed: %s\n' \
  "$c11_atomic_obj"

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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables \
  -Wno-implicit-function-declaration -I toolchain \
  -c "$exit_c" -o "$exit_obj"
test -s "$exit_obj"
exit_dump="$build_dir/exit-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$exit_obj" >"$exit_dump"
grep -q 'call ' "$exit_dump"
printf 'real LLVM LNP64 clang exit object smoke passed: %s\n' "$exit_obj"

libc_process_impl_c="toolchain/liblnp64_process_min.c"
libc_process_impl_obj="$build_dir/liblnp64-process-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_process_impl_c" -o "$libc_process_impl_obj"
test -s "$libc_process_impl_obj"
libc_process_impl_dump="$build_dir/liblnp64-process-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_process_impl_obj" \
  >"$libc_process_impl_dump"
grep -q 'exit r' "$libc_process_impl_dump"
grep -q 'get_pcr r' "$libc_process_impl_dump"
grep -q 'fork r' "$libc_process_impl_dump"
grep -q 'wait_pid r' "$libc_process_impl_dump"
grep -q 'exec r' "$libc_process_impl_dump"
grep -q 'call ' "$libc_process_impl_dump"
printf 'real LLVM LNP64 clang minilibc process implementation object smoke passed: %s\n' \
  "$libc_process_impl_obj"

libc_errno_impl_c="toolchain/liblnp64_errno_min.c"
libc_errno_impl_obj="$build_dir/liblnp64-errno-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_errno_impl_c" -o "$libc_errno_impl_obj"
test -s "$libc_errno_impl_obj"
libc_errno_impl_dump="$build_dir/liblnp64-errno-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_errno_impl_obj" \
  >"$libc_errno_impl_dump"
grep -q 'errno_get r' "$libc_errno_impl_dump"
grep -q 'errno_set r' "$libc_errno_impl_dump"
grep -q 'ret' "$libc_errno_impl_dump"
printf 'real LLVM LNP64 clang minilibc errno implementation object smoke passed: %s\n' \
  "$libc_errno_impl_obj"

libc_startup_impl_c="toolchain/liblnp64_startup_min.c"
libc_startup_impl_obj="$build_dir/liblnp64-startup-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_startup_impl_c" -o "$libc_startup_impl_obj"
test -s "$libc_startup_impl_obj"
libc_startup_impl_dump="$build_dir/liblnp64-startup-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_startup_impl_obj" \
  >"$libc_startup_impl_dump"
grep -q 'env_get r' "$libc_startup_impl_dump"
grep -q 'ret' "$libc_startup_impl_dump"
printf 'real LLVM LNP64 clang minilibc startup implementation object smoke passed: %s\n' \
  "$libc_startup_impl_obj"

libc_random_impl_c="toolchain/liblnp64_random_min.c"
libc_random_impl_obj="$build_dir/liblnp64-random-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c "$libc_random_impl_c" -o "$libc_random_impl_obj"
test -s "$libc_random_impl_obj"
libc_random_impl_dump="$build_dir/liblnp64-random-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_random_impl_obj" \
  >"$libc_random_impl_dump"
grep -q '<random>:' "$libc_random_impl_dump"
grep -q '<srandom>:' "$libc_random_impl_dump"
grep -q '<initstate>:' "$libc_random_impl_dump"
grep -q '<setstate>:' "$libc_random_impl_dump"
printf 'real LLVM LNP64 clang minilibc random implementation object smoke passed: %s\n' \
  "$libc_random_impl_obj"

libc_time_impl_c="toolchain/liblnp64_time_min.c"
libc_time_impl_obj="$build_dir/liblnp64-time-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c "$libc_time_impl_c" -o "$libc_time_impl_obj"
test -s "$libc_time_impl_obj"
libc_time_impl_dump="$build_dir/liblnp64-time-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_time_impl_obj" \
  >"$libc_time_impl_dump"
grep -q 'get_pcr r' "$libc_time_impl_dump"
grep -q 'yield' "$libc_time_impl_dump"
grep -q 'object_ctl r' "$libc_time_impl_dump"
grep -q 'push r' "$libc_time_impl_dump"
grep -q 'ret' "$libc_time_impl_dump"
printf 'real LLVM LNP64 clang minilibc time implementation object smoke passed: %s\n' \
  "$libc_time_impl_obj"

libc_vma_impl_c="toolchain/liblnp64_vma_min.c"
libc_vma_impl_obj="$build_dir/liblnp64-vma-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_vma_impl_c" -o "$libc_vma_impl_obj"
test -s "$libc_vma_impl_obj"
libc_vma_impl_dump="$build_dir/liblnp64-vma-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_vma_impl_obj" \
  >"$libc_vma_impl_dump"
grep -q 'mmap r' "$libc_vma_impl_dump"
grep -q 'mprotect r' "$libc_vma_impl_dump"
grep -q 'munmap r' "$libc_vma_impl_dump"
grep -q 'call ' "$libc_vma_impl_dump"
printf 'real LLVM LNP64 clang minilibc VMA implementation object smoke passed: %s\n' \
  "$libc_vma_impl_obj"

libc_futex_impl_c="toolchain/liblnp64_futex_min.c"
libc_futex_impl_obj="$build_dir/liblnp64-futex-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_futex_impl_c" -o "$libc_futex_impl_obj"
test -s "$libc_futex_impl_obj"
libc_futex_impl_dump="$build_dir/liblnp64-futex-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_futex_impl_obj" \
  >"$libc_futex_impl_dump"
grep -q 'futex_wait r' "$libc_futex_impl_dump"
grep -q 'futex_wake r' "$libc_futex_impl_dump"
grep -q 'ret' "$libc_futex_impl_dump"
printf 'real LLVM LNP64 clang minilibc futex implementation object smoke passed: %s\n' \
  "$libc_futex_impl_obj"

libc_pthread_impl_c="toolchain/liblnp64_pthread_min.c"
libc_pthread_impl_obj="$build_dir/liblnp64-pthread-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c "$libc_pthread_impl_c" -o "$libc_pthread_impl_obj"
test -s "$libc_pthread_impl_obj"
libc_pthread_impl_dump="$build_dir/liblnp64-pthread-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_pthread_impl_obj" \
  >"$libc_pthread_impl_dump"
grep -q 'clone.spawn r' "$libc_pthread_impl_dump"
grep -q 'thread_join r' "$libc_pthread_impl_dump"
grep -q 'get_pcr r' "$libc_pthread_impl_dump"
printf 'real LLVM LNP64 clang minilibc pthread implementation object smoke passed: %s\n' \
  "$libc_pthread_impl_obj"

libc_sem_impl_c="toolchain/liblnp64_sem_min.c"
libc_sem_impl_obj="$build_dir/liblnp64-sem-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c "$libc_sem_impl_c" -o "$libc_sem_impl_obj"
test -s "$libc_sem_impl_obj"
libc_sem_impl_dump="$build_dir/liblnp64-sem-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_sem_impl_obj" \
  >"$libc_sem_impl_dump"
grep -q 'futex_wait r' "$libc_sem_impl_dump"
grep -q 'futex_wake r' "$libc_sem_impl_dump"
grep -q 'lock.cmpxchg r' "$libc_sem_impl_dump"
printf 'real LLVM LNP64 clang minilibc semaphore implementation object smoke passed: %s\n' \
  "$libc_sem_impl_obj"

errno_c="$build_dir/errno-smoke.c"
cat >"$errno_c" <<'C'
int *__errno_location(void);
int lnp64_errno_store(int value);
const char *strerror(int value);

int main(void) {
  int *slot = __errno_location();
  if (*slot != 0)
    return 1;
  if (lnp64_errno_store(22) != 22)
    return 2;
  slot = __errno_location();
  if (*slot != 22)
    return 3;
  if (strerror(*slot)[0] != 'I')
    return 4;
  lnp64_errno_store(0);
  return *__errno_location();
}
C

errno_obj="$build_dir/errno-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$errno_c" -o "$errno_obj"
test -s "$errno_obj"
errno_dump="$build_dir/errno-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$errno_obj" >"$errno_dump"
grep -q 'call ' "$errno_dump"
printf 'real LLVM LNP64 clang errno object smoke passed: %s\n' "$errno_obj"

argc_c="$build_dir/argc-smoke.c"
cat >"$argc_c" <<'C'
int main(int argc, char **argv) {
  (void)argv;
  return argc;
}
C

argc_obj="$build_dir/argc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$argc_c" -o "$argc_obj"
test -s "$argc_obj"
argc_dump="$build_dir/argc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$argc_obj" >"$argc_dump"
grep -q '<main>:' "$argc_dump"
grep -q 'ret' "$argc_dump"
printf 'real LLVM LNP64 clang argc object smoke passed: %s\n' "$argc_obj"

startup_c="$build_dir/startup-smoke.c"
cat >"$startup_c" <<'C'
int main(int argc, char **argv, char **envp) {
  if (argc != 0)
    return 1;
  if (!argv || argv[0] != 0)
    return 2;
  if (!envp || envp[0] != 0)
    return 3;
  return 0;
}
C

startup_obj="$build_dir/startup-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$startup_c" -o "$startup_obj"
test -s "$startup_obj"
startup_dump="$build_dir/startup-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$startup_obj" >"$startup_dump"
grep -q '<main>:' "$startup_dump"
grep -q 'ld ' "$startup_dump"
printf 'real LLVM LNP64 clang startup argv/envp object smoke passed: %s\n' \
  "$startup_obj"

libc_stdio_impl_c="toolchain/liblnp64_stdio_min.c"
libc_stdio_impl_obj="$build_dir/liblnp64-stdio-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -c "$libc_stdio_impl_c" -o "$libc_stdio_impl_obj"
test -s "$libc_stdio_impl_obj"
libc_stdio_impl_dump="$build_dir/liblnp64-stdio-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_stdio_impl_obj" \
  >"$libc_stdio_impl_dump"
grep -q '<vsnprintf>:' "$libc_stdio_impl_dump"
grep -q '<snprintf>:' "$libc_stdio_impl_dump"
grep -q '<tmpfile>:' "$libc_stdio_impl_dump"
grep -q '<fileno>:' "$libc_stdio_impl_dump"
printf 'real LLVM LNP64 clang minilibc stdio implementation object smoke passed: %s\n' \
  "$libc_stdio_impl_obj"

getauxval_c="$build_dir/getauxval-smoke.c"
cat >"$getauxval_c" <<'C'
#include <sys/auxv.h>

int main(void) {
  if (getauxval(AT_PAGESZ) != 4096)
    return 1;
  if (getauxval(AT_HWCAP) == 0)
    return 2;
  if (getauxval(9999) != 0)
    return 3;
  return 0;
}
C

getauxval_obj="$build_dir/getauxval-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$getauxval_c" -o "$getauxval_obj"
test -s "$getauxval_obj"
getauxval_dump="$build_dir/getauxval-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$getauxval_obj" \
  >"$getauxval_dump"
grep -q 'call ' "$getauxval_dump"
printf 'real LLVM LNP64 clang getauxval object smoke passed: %s\n' \
  "$getauxval_obj"

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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
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
#include <ctype.h>
#include <string.h>

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
  if (strcmp("abc", "abc") != 0)
    return 16;
  if (strcmp("abc", "abd") >= 0)
    return 17;
  if (strncmp("abcdef", "abcxyz", 3) != 0)
    return 18;
  if (strncmp("abcdef", "abcxyz", 4) >= 0)
    return 19;
  if (strcpy(dst, "xy") != dst || dst[0] != 'x' || dst[1] != 'y' || dst[2] != 0)
    return 20;
  char bounded[10];
  if (strncpy(bounded, "abc", 6) != bounded)
    return 21;
  if (bounded[0] != 'a' || bounded[2] != 'c' || bounded[3] != 0)
    return 22;
  if (bounded[4] != 0 || bounded[5] != 0)
    return 23;
  if (strncpy(bounded, "abcdef", 3) != bounded)
    return 24;
  if (bounded[0] != 'a' || bounded[2] != 'c' || bounded[3] != 0)
    return 25;
  if (strcpy(bounded, "xy") != bounded)
    return 26;
  if (strncat(bounded, "zpq", 1) != bounded)
    return 27;
  if (strcmp(bounded, "xyz") != 0)
    return 28;
  if (strchr("abcd", 'c') == 0 || *strchr("abcd", 'c') != 'c')
    return 29;
  if (strchr("abcd", 'z') != 0)
    return 30;
  const char *scan = "abca";
  if (strrchr(scan, 'a') == 0 || *strrchr(scan, 'a') != 'a')
    return 31;
  if (strrchr(scan, 'a') != scan + 3)
    return 32;
  if (strrchr(scan, 0) != scan + 4)
    return 33;
  if (strstr("abcde", "bcd") == 0 || *strstr("abcde", "bcd") != 'b')
    return 34;
  if (strstr("abcde", "bd") != 0)
    return 35;
  if (strspn("abc123", "abc") != 3)
    return 36;
  if (strcspn("abc123", "321") != 3)
    return 37;
  if (strpbrk("abc123", "29") == 0 || *strpbrk("abc123", "29") != '2')
    return 38;
  unsigned char high[6] = {1, 2, 127, 128, 255, 0};
  char reject[2] = {(char)255, 0};
  if (strcspn((char *)high, reject) != 4)
    return 39;
  char tokens[16];
  strcpy(tokens, ",one,two");
  char *tok = strtok(tokens, ",");
  if (tok == 0 || strcmp(tok, "one") != 0)
    return 40;
  tok = strtok(0, ",");
  if (tok == 0 || strcmp(tok, "two") != 0)
    return 41;
  if (strtok(0, ",") != 0)
    return 42;
  char small[5];
  if (strlcpy(small, "abcdef", sizeof(small)) != 6)
    return 43;
  if (strcmp(small, "abcd") != 0)
    return 44;
  strcpy(small, "ab");
  if (strlcat(small, "cdef", sizeof(small)) != 6)
    return 45;
  if (strcmp(small, "abcd") != 0)
    return 46;
  if (strlcat(small, "z", 2) != 3)
    return 47;
  char hay[7] = {'a', 'b', 0, 'c', 'd', 'e', 0};
  char needle[2] = {0, 'c'};
  if (memmem(hay, 7, needle, 2) != hay + 2)
    return 48;
  if (memmem(hay, 7, "de", 2) != hay + 4)
    return 49;
  if (memmem(hay, 7, "zz", 2) != 0)
    return 50;
  if (memmem(hay, 7, "", 0) != hay)
    return 51;
  if (!isalpha('Q') || !islower('q') || !isupper('Q'))
    return 52;
  if (!isdigit('7') || !isalnum('7') || !isxdigit('f'))
    return 53;
  if (!isspace('\n') || isspace('x'))
    return 54;
  if (tolower('Q') != 'q' || toupper('q') != 'Q')
    return 55;
  if (tolower('7') != '7' || toupper('7') != '7')
    return 56;
  return 0;
}
C

libc_string_obj="$build_dir/libc-string-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
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

libc_string_impl_c="toolchain/liblnp64_string_min.c"
libc_string_impl_obj="$build_dir/liblnp64-string-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_string_impl_c" -o "$libc_string_impl_obj"
test -s "$libc_string_impl_obj"
libc_string_impl_dump="$build_dir/liblnp64-string-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_string_impl_obj" \
  >"$libc_string_impl_dump"
grep -q 'ld.b r' "$libc_string_impl_dump"
grep -q 'st.b r' "$libc_string_impl_dump"
grep -q 'ret' "$libc_string_impl_dump"
printf 'real LLVM LNP64 clang minilibc string implementation object smoke passed: %s\n' \
  "$libc_string_impl_obj"

libc_test_print_obj="$build_dir/libc-test-print-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/print.c -o "$libc_test_print_obj"
test -s "$libc_test_print_obj"
printf 'real LLVM LNP64 clang libc-test harness object smoke passed: %s\n' \
  "$libc_test_print_obj"

libc_test_argv_obj="$build_dir/libc-test-argv-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/argv.c \
  -o "$libc_test_argv_obj"
test -s "$libc_test_argv_obj"
libc_test_argv_dump="$build_dir/libc-test-argv-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_argv_obj" \
  >"$libc_test_argv_dump"
grep -q 'call ' "$libc_test_argv_dump"
printf 'real LLVM LNP64 clang libc-test argv object smoke passed: %s\n' \
  "$libc_test_argv_obj"

libc_test_env_obj="$build_dir/libc-test-env-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/env.c \
  -o "$libc_test_env_obj"
test -s "$libc_test_env_obj"
libc_test_env_dump="$build_dir/libc-test-env-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_env_obj" \
  >"$libc_test_env_dump"
grep -q 'call ' "$libc_test_env_dump"
printf 'real LLVM LNP64 clang libc-test env object smoke passed: %s\n' \
  "$libc_test_env_obj"

libc_test_random_obj="$build_dir/libc-test-random-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/random.c \
  -o "$libc_test_random_obj"
test -s "$libc_test_random_obj"
libc_test_random_dump="$build_dir/libc-test-random-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_random_obj" \
  >"$libc_test_random_dump"
grep -q 'call ' "$libc_test_random_dump"
printf 'real LLVM LNP64 clang libc-test random object smoke passed: %s\n' \
  "$libc_test_random_obj"

libc_test_ctype_obj="$build_dir/libc-test-ctype-bounded-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/ctype_bounded.c \
  -o "$libc_test_ctype_obj"
test -s "$libc_test_ctype_obj"
libc_test_ctype_dump="$build_dir/libc-test-ctype-bounded-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_ctype_obj" \
  >"$libc_test_ctype_dump"
grep -q 'call ' "$libc_test_ctype_dump"
printf 'real LLVM LNP64 clang libc-test ctype_bounded object smoke passed: %s\n' \
  "$libc_test_ctype_obj"

libc_test_string_obj="$build_dir/libc-test-string-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/string.c \
  -o "$libc_test_string_obj"
test -s "$libc_test_string_obj"
libc_test_string_dump="$build_dir/libc-test-string-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_string_obj" \
  >"$libc_test_string_dump"
grep -q 'call ' "$libc_test_string_dump"
printf 'real LLVM LNP64 clang libc-test string object smoke passed: %s\n' \
  "$libc_test_string_obj"

libc_test_memcpy_bounded_obj="$build_dir/libc-test-string-memcpy-bounded-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/string_memcpy_bounded.c \
  -o "$libc_test_memcpy_bounded_obj"
test -s "$libc_test_memcpy_bounded_obj"
libc_test_memcpy_bounded_dump="$build_dir/libc-test-string-memcpy-bounded-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_memcpy_bounded_obj" \
  >"$libc_test_memcpy_bounded_dump"
grep -q 'call ' "$libc_test_memcpy_bounded_dump"
printf 'real LLVM LNP64 clang libc-test string_memcpy_bounded object smoke passed: %s\n' \
  "$libc_test_memcpy_bounded_obj"

libc_test_memmove_bounded_obj="$build_dir/libc-test-string-memmove-bounded-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/string_memmove_bounded.c \
  -o "$libc_test_memmove_bounded_obj"
test -s "$libc_test_memmove_bounded_obj"
libc_test_memmove_bounded_dump="$build_dir/libc-test-string-memmove-bounded-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_memmove_bounded_obj" \
  >"$libc_test_memmove_bounded_dump"
grep -q 'call ' "$libc_test_memmove_bounded_dump"
printf 'real LLVM LNP64 clang libc-test string_memmove_bounded object smoke passed: %s\n' \
  "$libc_test_memmove_bounded_obj"

libc_test_memmem_obj="$build_dir/libc-test-string-memmem-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/string_memmem.c \
  -o "$libc_test_memmem_obj"
test -s "$libc_test_memmem_obj"
libc_test_memmem_dump="$build_dir/libc-test-string-memmem-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_memmem_obj" \
  >"$libc_test_memmem_dump"
grep -q 'call ' "$libc_test_memmem_dump"
printf 'real LLVM LNP64 clang libc-test string_memmem object smoke passed: %s\n' \
  "$libc_test_memmem_obj"

libc_test_strchr_obj="$build_dir/libc-test-string-strchr-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/string_strchr.c \
  -o "$libc_test_strchr_obj"
test -s "$libc_test_strchr_obj"
libc_test_strchr_dump="$build_dir/libc-test-string-strchr-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_strchr_obj" \
  >"$libc_test_strchr_dump"
grep -q 'call ' "$libc_test_strchr_dump"
printf 'real LLVM LNP64 clang libc-test string_strchr object smoke passed: %s\n' \
  "$libc_test_strchr_obj"

libc_test_strcspn_obj="$build_dir/libc-test-string-strcspn-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/string_strcspn.c \
  -o "$libc_test_strcspn_obj"
test -s "$libc_test_strcspn_obj"
libc_test_strcspn_dump="$build_dir/libc-test-string-strcspn-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_strcspn_obj" \
  >"$libc_test_strcspn_dump"
grep -q 'call ' "$libc_test_strcspn_dump"
printf 'real LLVM LNP64 clang libc-test string_strcspn object smoke passed: %s\n' \
  "$libc_test_strcspn_obj"

libc_test_strstr_obj="$build_dir/libc-test-string-strstr-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/string_strstr.c \
  -o "$libc_test_strstr_obj"
test -s "$libc_test_strstr_obj"
libc_test_strstr_dump="$build_dir/libc-test-string-strstr-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_strstr_obj" \
  >"$libc_test_strstr_dump"
grep -q 'call ' "$libc_test_strstr_dump"
printf 'real LLVM LNP64 clang libc-test string_strstr object smoke passed: %s\n' \
  "$libc_test_strstr_obj"

libc_test_udiv_obj="$build_dir/libc-test-udiv-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/udiv.c \
  -o "$libc_test_udiv_obj"
test -s "$libc_test_udiv_obj"
libc_test_udiv_dump="$build_dir/libc-test-udiv-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_udiv_obj" \
  >"$libc_test_udiv_dump"
grep -q 'udiv r' "$libc_test_udiv_dump"
grep -q 'urem r' "$libc_test_udiv_dump"
printf 'real LLVM LNP64 clang libc-test udiv object smoke passed: %s\n' \
  "$libc_test_udiv_obj"

libc_test_basename_obj="$build_dir/libc-test-basename-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/basename.c \
  -o "$libc_test_basename_obj"
test -s "$libc_test_basename_obj"
libc_test_basename_dump="$build_dir/libc-test-basename-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_basename_obj" \
  >"$libc_test_basename_dump"
grep -q 'call ' "$libc_test_basename_dump"
printf 'real LLVM LNP64 clang libc-test basename object smoke passed: %s\n' \
  "$libc_test_basename_obj"

libc_test_dirname_obj="$build_dir/libc-test-dirname-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/dirname.c \
  -o "$libc_test_dirname_obj"
test -s "$libc_test_dirname_obj"
libc_test_dirname_dump="$build_dir/libc-test-dirname-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_dirname_obj" \
  >"$libc_test_dirname_dump"
grep -q 'call ' "$libc_test_dirname_dump"
printf 'real LLVM LNP64 clang libc-test dirname object smoke passed: %s\n' \
  "$libc_test_dirname_obj"

libc_test_strtol_obj="$build_dir/libc-test-strtol-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/strtol.c \
  -o "$libc_test_strtol_obj"
test -s "$libc_test_strtol_obj"
libc_test_strtol_dump="$build_dir/libc-test-strtol-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_strtol_obj" \
  >"$libc_test_strtol_dump"
grep -q 'call ' "$libc_test_strtol_dump"
printf 'real LLVM LNP64 clang libc-test strtol object smoke passed: %s\n' \
  "$libc_test_strtol_obj"

libc_test_clock_gettime_obj="$build_dir/libc-test-clock-gettime-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/clock_gettime.c \
  -o "$libc_test_clock_gettime_obj"
test -s "$libc_test_clock_gettime_obj"
libc_test_clock_gettime_dump="$build_dir/libc-test-clock-gettime-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_clock_gettime_obj" \
  >"$libc_test_clock_gettime_dump"
grep -q 'call ' "$libc_test_clock_gettime_dump"
printf 'real LLVM LNP64 clang libc-test clock_gettime object smoke passed: %s\n' \
  "$libc_test_clock_gettime_obj"

libc_test_access_bounded_obj="$build_dir/libc-test-access-bounded-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/access_bounded.c \
  -o "$libc_test_access_bounded_obj"
test -s "$libc_test_access_bounded_obj"
libc_test_access_bounded_dump="$build_dir/libc-test-access-bounded-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_access_bounded_obj" \
  >"$libc_test_access_bounded_dump"
grep -q 'call ' "$libc_test_access_bounded_dump"
printf 'real LLVM LNP64 clang libc-test access_bounded object smoke passed: %s\n' \
  "$libc_test_access_bounded_obj"

libc_test_stat_obj="$build_dir/libc-test-stat-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/stat.c \
  -o "$libc_test_stat_obj"
test -s "$libc_test_stat_obj"
libc_test_stat_dump="$build_dir/libc-test-stat-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_stat_obj" \
  >"$libc_test_stat_dump"
grep -q 'call ' "$libc_test_stat_dump"
printf 'real LLVM LNP64 clang libc-test stat object smoke passed: %s\n' \
  "$libc_test_stat_obj"

libc_test_utime_obj="$build_dir/libc-test-utime-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/utime.c \
  -o "$libc_test_utime_obj"
test -s "$libc_test_utime_obj"
libc_test_utime_dump="$build_dir/libc-test-utime-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_utime_obj" \
  >"$libc_test_utime_dump"
grep -q 'call ' "$libc_test_utime_dump"
printf 'real LLVM LNP64 clang libc-test utime object smoke passed: %s\n' \
  "$libc_test_utime_obj"

libc_test_ungetc_obj="$build_dir/libc-test-ungetc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/ungetc.c \
  -o "$libc_test_ungetc_obj"
test -s "$libc_test_ungetc_obj"
libc_test_ungetc_dump="$build_dir/libc-test-ungetc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_ungetc_obj" \
  >"$libc_test_ungetc_dump"
grep -q 'call ' "$libc_test_ungetc_dump"
printf 'real LLVM LNP64 clang libc-test ungetc object smoke passed: %s\n' \
  "$libc_test_ungetc_obj"

libc_test_fdopen_obj="$build_dir/libc-test-fdopen-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/fdopen.c \
  -o "$libc_test_fdopen_obj"
test -s "$libc_test_fdopen_obj"
libc_test_fdopen_dump="$build_dir/libc-test-fdopen-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_fdopen_obj" \
  >"$libc_test_fdopen_dump"
grep -q 'call ' "$libc_test_fdopen_dump"
printf 'real LLVM LNP64 clang libc-test fdopen object smoke passed: %s\n' \
  "$libc_test_fdopen_obj"

libc_test_fcntl_basic_obj="$build_dir/libc-test-fcntl-basic-bounded-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/fcntl_basic_bounded.c \
  -o "$libc_test_fcntl_basic_obj"
test -s "$libc_test_fcntl_basic_obj"
libc_test_fcntl_basic_dump="$build_dir/libc-test-fcntl-basic-bounded-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_fcntl_basic_obj" \
  >"$libc_test_fcntl_basic_dump"
grep -q 'call ' "$libc_test_fcntl_basic_dump"
printf 'real LLVM LNP64 clang libc-test fcntl_basic_bounded object smoke passed: %s\n' \
  "$libc_test_fcntl_basic_obj"

libc_test_pthread_tsd_obj="$build_dir/libc-test-pthread-tsd-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/pthread_tsd.c \
  -o "$libc_test_pthread_tsd_obj"
test -s "$libc_test_pthread_tsd_obj"
libc_test_pthread_tsd_dump="$build_dir/libc-test-pthread-tsd-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_pthread_tsd_obj" \
  >"$libc_test_pthread_tsd_dump"
grep -q 'call ' "$libc_test_pthread_tsd_dump"
printf 'real LLVM LNP64 clang libc-test pthread_tsd object smoke passed: %s\n' \
  "$libc_test_pthread_tsd_obj"

libc_test_sem_init_obj="$build_dir/libc-test-sem-init-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/sem_init.c \
  -o "$libc_test_sem_init_obj"
test -s "$libc_test_sem_init_obj"
libc_test_sem_init_dump="$build_dir/libc-test-sem-init-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_sem_init_obj" \
  >"$libc_test_sem_init_dump"
grep -q 'call ' "$libc_test_sem_init_dump"
printf 'real LLVM LNP64 clang libc-test sem_init object smoke passed: %s\n' \
  "$libc_test_sem_init_obj"

libc_test_qsort_bounded_obj="$build_dir/libc-test-qsort-bounded-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/qsort_bounded.c \
  -o "$libc_test_qsort_bounded_obj"
test -s "$libc_test_qsort_bounded_obj"
libc_test_qsort_bounded_dump="$build_dir/libc-test-qsort-bounded-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_qsort_bounded_obj" \
  >"$libc_test_qsort_bounded_dump"
grep -q 'call ' "$libc_test_qsort_bounded_dump"
printf 'real LLVM LNP64 clang libc-test qsort_bounded object smoke passed: %s\n' \
  "$libc_test_qsort_bounded_obj"

libc_test_search_insque_obj="$build_dir/libc-test-search-insque-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/search_insque.c \
  -o "$libc_test_search_insque_obj"
test -s "$libc_test_search_insque_obj"
libc_test_search_insque_dump="$build_dir/libc-test-search-insque-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_search_insque_obj" \
  >"$libc_test_search_insque_dump"
grep -q 'call ' "$libc_test_search_insque_dump"
printf 'real LLVM LNP64 clang libc-test search_insque object smoke passed: %s\n' \
  "$libc_test_search_insque_obj"

libc_test_search_lsearch_obj="$build_dir/libc-test-search-lsearch-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/functional/search_lsearch.c \
  -o "$libc_test_search_lsearch_obj"
test -s "$libc_test_search_lsearch_obj"
libc_test_search_lsearch_dump="$build_dir/libc-test-search-lsearch-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_search_lsearch_obj" \
  >"$libc_test_search_lsearch_dump"
grep -q 'call ' "$libc_test_search_lsearch_dump"
printf 'real LLVM LNP64 clang libc-test search_lsearch object smoke passed: %s\n' \
  "$libc_test_search_lsearch_obj"

libc_test_malloc_0_obj="$build_dir/libc-test-malloc-0-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/regression/malloc-0.c \
  -o "$libc_test_malloc_0_obj"
test -s "$libc_test_malloc_0_obj"
libc_test_malloc_0_dump="$build_dir/libc-test-malloc-0-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_malloc_0_obj" \
  >"$libc_test_malloc_0_dump"
grep -q 'call ' "$libc_test_malloc_0_dump"
printf 'real LLVM LNP64 clang libc-test malloc-0 object smoke passed: %s\n' \
  "$libc_test_malloc_0_obj"

libc_test_fgets_eof_obj="$build_dir/libc-test-fgets-eof-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain/include \
  -I third_party/libc-test/functional \
  -c third_party/libc-test/regression/fgets-eof.c \
  -o "$libc_test_fgets_eof_obj"
test -s "$libc_test_fgets_eof_obj"
libc_test_fgets_eof_dump="$build_dir/libc-test-fgets-eof-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_test_fgets_eof_obj" \
  >"$libc_test_fgets_eof_dump"
grep -q 'call ' "$libc_test_fgets_eof_dump"
printf 'real LLVM LNP64 clang libc-test fgets-eof object smoke passed: %s\n' \
  "$libc_test_fgets_eof_obj"

convert_c="$build_dir/convert-smoke.c"
cat >"$convert_c" <<'C'
#include <errno.h>
#include <stdlib.h>

int main(void) {
  char *end;
  const char *s;

  if (atoi("123") != 123)
    return 1;
  if (atol("-42") != -42)
    return 2;
  s = "  15437";
  if (strtol(s, &end, 8) != 015437)
    return 3;
  if (end - s != 7)
    return 4;
  s = "0xz";
  if (strtol(s, &end, 16) != 0)
    return 5;
  if (end - s != 1)
    return 6;
  s = "0x1234";
  if (strtol(s, &end, 16) != 0x1234)
    return 7;
  if (end - s != 6)
    return 8;
  if (strtol("z", 0, 36) != 35)
    return 9;
  if (strtol("00010010001101000101011001111000", 0, 2) != 0x12345678)
    return 10;
  errno = 0;
  s = "123";
  if (strtol(s, &end, 37) != 0)
    return 11;
  if (end != s)
    return 12;
  if (errno != 22)
    return 13;
  errno = 0;
  s = "9223372036854775808";
  if (strtol(s, &end, 0) <= 0)
    return 14;
  if (end - s != 19)
    return 15;
  if (errno != 34)
    return 16;
  errno = 0;
  s = "-9223372036854775809";
  if (strtoll(s, &end, 0) >= 0)
    return 17;
  if (end - s != 20)
    return 18;
  if (errno != 34)
    return 19;
  errno = 0;
  s = "-1";
  if (strtoull(s, &end, 0) != ~0ULL)
    return 20;
  if (end - s != 2)
    return 21;
  if (errno != 0)
    return 22;
  s = "18446744073709551616";
  if (strtoull(s, &end, 0) != ~0ULL)
    return 23;
  if (end - s != 20)
    return 24;
  if (errno != 34)
    return 25;
  if (strtoul("4294967295", 0, 0) != 4294967295UL)
    return 26;
  return 0;
}
C

convert_obj="$build_dir/convert-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$convert_c" -o "$convert_obj"
test -s "$convert_obj"
convert_dump="$build_dir/convert-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$convert_obj" \
  >"$convert_dump"
grep -q 'call ' "$convert_dump"
printf 'real LLVM LNP64 clang numeric conversion object smoke passed: %s\n' \
  "$convert_obj"

libc_convert_impl_c="toolchain/liblnp64_convert_min.c"
libc_convert_impl_obj="$build_dir/liblnp64-convert-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_convert_impl_c" -o "$libc_convert_impl_obj"
test -s "$libc_convert_impl_obj"
libc_convert_impl_dump="$build_dir/liblnp64-convert-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_convert_impl_obj" \
  >"$libc_convert_impl_dump"
grep -q 'call ' "$libc_convert_impl_dump"
grep -q 'ret' "$libc_convert_impl_dump"
printf 'real LLVM LNP64 clang minilibc numeric conversion implementation object smoke passed: %s\n' \
  "$libc_convert_impl_obj"

path_c="$build_dir/path-smoke.c"
cat >"$path_c" <<'C'
int strcmp(const char *lhs, const char *rhs);
char *strcpy(char *dst, const char *src);
char *basename(char *path);
char *dirname(char *path);

static int check_basename(const char *path, const char *want) {
  char tmp[100];
  char *got = basename(strcpy(tmp, path));
  return strcmp(got, want) == 0;
}

static int check_dirname(const char *path, const char *want) {
  char tmp[100];
  char *got = dirname(strcpy(tmp, path));
  return strcmp(got, want) == 0;
}

int main(void) {
  if (strcmp(basename(0), ".") != 0)
    return 1;
  if (!check_basename("", "."))
    return 2;
  if (!check_basename("/usr/lib", "lib"))
    return 3;
  if (!check_basename("/usr/", "usr"))
    return 4;
  if (!check_basename("usr/", "usr"))
    return 5;
  if (!check_basename("/", "/"))
    return 6;
  if (!check_basename("///", "/"))
    return 7;
  if (!check_basename("//usr//lib//", "lib"))
    return 8;
  if (!check_basename(".", "."))
    return 9;
  if (!check_basename("..", ".."))
    return 10;

  if (strcmp(dirname(0), ".") != 0)
    return 11;
  if (!check_dirname("", "."))
    return 12;
  if (!check_dirname("/usr/lib", "/usr"))
    return 13;
  if (!check_dirname("/usr/", "/"))
    return 14;
  if (!check_dirname("usr", "."))
    return 15;
  if (!check_dirname("usr/", "."))
    return 16;
  if (!check_dirname("/", "/"))
    return 17;
  if (!check_dirname("///", "/"))
    return 18;
  if (!check_dirname(".", "."))
    return 19;
  if (!check_dirname("..", "."))
    return 20;
  return 0;
}
C

path_obj="$build_dir/path-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$path_c" -o "$path_obj"
test -s "$path_obj"
path_dump="$build_dir/path-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$path_obj" \
  >"$path_dump"
grep -q 'call ' "$path_dump"
printf 'real LLVM LNP64 clang path helper object smoke passed: %s\n' \
  "$path_obj"

libc_path_impl_c="toolchain/liblnp64_path_min.c"
libc_path_impl_obj="$build_dir/liblnp64-path-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_path_impl_c" -o "$libc_path_impl_obj"
test -s "$libc_path_impl_obj"
libc_path_impl_dump="$build_dir/liblnp64-path-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_path_impl_obj" \
  >"$libc_path_impl_dump"
grep -q 'ld.b r' "$libc_path_impl_dump"
grep -q 'st.b r' "$libc_path_impl_dump"
grep -q 'ret' "$libc_path_impl_dump"
printf 'real LLVM LNP64 clang minilibc path implementation object smoke passed: %s\n' \
  "$libc_path_impl_obj"

search_c="$build_dir/search-smoke.c"
cat >"$search_c" <<'C'
typedef unsigned long size_t;
int strcmp(const char *lhs, const char *rhs);
void *lfind(const void *key, const void *base, size_t *nelp, size_t width,
            int (*compar)(const void *, const void *));
void *lsearch(const void *key, void *base, size_t *nelp, size_t width,
              int (*compar)(const void *, const void *));
void insque(void *elem, void *pred);
void remque(void *elem);

struct node {
  struct node *n;
  struct node *p;
  int value;
};

static char tab[16][16];
static const char key_empty[16] = "";
static const char key_a[16] = "a";
static const char key_b[16] = "b";
static const char key_abc[16] = "abc";
static const char key_c[16] = "c";
static const char key_j[16] = "j";
static size_t nel;

static int set(const char *key) {
  char *r = lsearch(key, tab, &nel, 16, (int (*)(const void *, const void *))strcmp);
  return strcmp(r, key) == 0;
}

static char *get(const char *key) {
  return lfind(key, tab, &nel, 16, (int (*)(const void *, const void *))strcmp);
}

int main(void) {
  size_t before;
  struct node nodes[10];
  struct node *q;
  struct node *p;
  int i;

  if (!set(key_empty) || !set(key_a) || !set(key_b) || !set(key_abc))
    return 1;
  if (!get(key_a))
    return 2;
  if (get(key_c))
    return 3;
  before = nel;
  if (!set(key_b))
    return 4;
  if (nel != before)
    return 5;
  before = nel;
  if (!set(key_j))
    return 6;
  if (nel != before + 1)
    return 7;

  for (i = 0; i < 10; i = i + 1) {
    nodes[i].n = 0;
    nodes[i].p = 0;
    nodes[i].value = i;
  }
  q = &nodes[0];
  insque(q, 0);
  for (i = 1; i < 10; i = i + 1) {
    insque(&nodes[i], q);
    q = q->n;
  }
  p = q;
  while (q) {
    i = i - 1;
    if (q->value != i)
      return 8;
    q = q->p;
  }
  remque(p->p);
  if (p->p->value != p->value - 2)
    return 9;
  if (p->p->n->value != p->value)
    return 10;
  return 0;
}
C

search_obj="$build_dir/search-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$search_c" -o "$search_obj"
test -s "$search_obj"
search_dump="$build_dir/search-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$search_obj" \
  >"$search_dump"
grep -q 'call ' "$search_dump"
printf 'real LLVM LNP64 clang search helper object smoke passed: %s\n' \
  "$search_obj"

libc_search_impl_c="toolchain/liblnp64_search_min.c"
libc_search_impl_obj="$build_dir/liblnp64-search-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_search_impl_c" -o "$libc_search_impl_obj"
test -s "$libc_search_impl_obj"
libc_search_impl_dump="$build_dir/liblnp64-search-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_search_impl_obj" \
  >"$libc_search_impl_dump"
grep -q 'call ' "$libc_search_impl_dump"
grep -q 'ret' "$libc_search_impl_dump"
printf 'real LLVM LNP64 clang minilibc search implementation object smoke passed: %s\n' \
  "$libc_search_impl_obj"

sort_c="$build_dir/sort-smoke.c"
cat >"$sort_c" <<'C'
typedef unsigned long size_t;
typedef unsigned long uint64_t;
int strcmp(const char *lhs, const char *rhs);
int memcmp(const void *lhs, const void *rhs, size_t len);
void qsort(void *base, size_t nmemb, size_t width,
           int (*compar)(const void *, const void *));

static int scmp(const void *lhs, const void *rhs) {
  const char *const *a = lhs;
  const char *const *b = rhs;
  return strcmp(*a, *b);
}

static int icmp(const void *lhs, const void *rhs) {
  const int *a = lhs;
  const int *b = rhs;
  return *a - *b;
}

static int ccmp(const void *lhs, const void *rhs) {
  const char *a = lhs;
  const char *b = rhs;
  return *a - *b;
}

static int cmp64(const void *lhs, const void *rhs) {
  const uint64_t *a = lhs;
  const uint64_t *b = rhs;
  if (*a < *b)
    return -1;
  return *a != *b;
}

int main(void) {
  const char *names[6] = {"Bob", "Alice", "John", "Ceres", "Helga", "Drepper"};
  const char *names_sorted[6] = {"Alice", "Bob", "Ceres", "Drepper", "Helga", "John"};
  int nums[8] = {879045, 394, 33434, 232323, 4334, 5454, 343, 45545};
  int nums_sorted[8] = {343, 394, 4334, 5454, 33434, 45545, 232323, 879045};
  char chars[] = "4517263";
  uint64_t wide[6] = {55, 3, 1024, 7, 7, 19};
  uint64_t wide_sorted[6] = {3, 7, 7, 19, 55, 1024};
  int i;

  qsort(names, 6, sizeof names[0], scmp);
  for (i = 0; i < 6; i = i + 1) {
    if (strcmp(names[i], names_sorted[i]) != 0)
      return 1;
  }
  qsort(nums, 8, sizeof nums[0], icmp);
  for (i = 0; i < 8; i = i + 1) {
    if (nums[i] != nums_sorted[i])
      return 2;
  }
  qsort(chars, sizeof chars - 1, 1, ccmp);
  if (memcmp(chars, "1234567", sizeof chars) != 0)
    return 3;
  qsort(wide, 6, sizeof wide[0], cmp64);
  for (i = 0; i < 6; i = i + 1) {
    if (wide[i] != wide_sorted[i])
      return 4;
  }
  qsort(chars, 0, 1, ccmp);
  qsort(chars, sizeof chars - 1, 0, ccmp);
  return 0;
}
C

sort_obj="$build_dir/sort-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$sort_c" -o "$sort_obj"
test -s "$sort_obj"
sort_dump="$build_dir/sort-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$sort_obj" \
  >"$sort_dump"
grep -q 'call ' "$sort_dump"
printf 'real LLVM LNP64 clang sort helper object smoke passed: %s\n' \
  "$sort_obj"

libc_sort_impl_c="toolchain/liblnp64_sort_min.c"
libc_sort_impl_obj="$build_dir/liblnp64-sort-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_sort_impl_c" -o "$libc_sort_impl_obj"
test -s "$libc_sort_impl_obj"
libc_sort_impl_dump="$build_dir/liblnp64-sort-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_sort_impl_obj" \
  >"$libc_sort_impl_dump"
grep -q 'st.b r' "$libc_sort_impl_dump"
grep -q 'ret' "$libc_sort_impl_dump"
printf 'real LLVM LNP64 clang minilibc sort implementation object smoke passed: %s\n' \
  "$libc_sort_impl_obj"

libc_alloc_impl_c="toolchain/liblnp64_alloc_min.c"
libc_alloc_impl_obj="$build_dir/liblnp64-alloc-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_alloc_impl_c" -o "$libc_alloc_impl_obj"
test -s "$libc_alloc_impl_obj"
libc_alloc_impl_dump="$build_dir/liblnp64-alloc-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_alloc_impl_obj" \
  >"$libc_alloc_impl_dump"
grep -q 'alloc r' "$libc_alloc_impl_dump"
grep -q 'alloc_size r' "$libc_alloc_impl_dump"
grep -q 'free r' "$libc_alloc_impl_dump"
grep -q 'call ' "$libc_alloc_impl_dump"
printf 'real LLVM LNP64 clang minilibc allocation implementation object smoke passed: %s\n' \
  "$libc_alloc_impl_obj"

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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
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
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
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

long read(long fd, void *buf, size_t len);

int main(void) {
  char byte = 0;
  return read(0, &byte, 0);
}
C

read_obj="$build_dir/read-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$read_c" -o "$read_obj"
test -s "$read_obj"
read_dump="$build_dir/read-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$read_obj" \
  >"$read_dump"
grep -q 'call ' "$read_dump"
printf 'real LLVM LNP64 clang read object smoke passed: %s\n' \
  "$read_obj"

write_c="$build_dir/write-smoke.c"
cat >"$write_c" <<'C'
typedef unsigned long size_t;

long write(long fd, const void *buf, size_t len);

int main(void) {
  const char msg[] = "fd write ok\n";
  return write(1, msg, sizeof(msg) - 1) == (long)(sizeof(msg) - 1) ? 0 : 1;
}
C

write_obj="$build_dir/write-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$write_c" -o "$write_obj"
test -s "$write_obj"
write_dump="$build_dir/write-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$write_obj" \
  >"$write_dump"
grep -q 'call ' "$write_dump"
printf 'real LLVM LNP64 clang write object smoke passed: %s\n' \
  "$write_obj"

userland_ucat_obj="$build_dir/userland-ucat-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/ucat_clang.c -o "$userland_ucat_obj"
test -s "$userland_ucat_obj"
userland_ucat_dump="$build_dir/userland-ucat-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$userland_ucat_obj" \
  >"$userland_ucat_dump"
grep -q 'call ' "$userland_ucat_dump"
printf 'real LLVM LNP64 clang userland ucat object smoke passed: %s\n' \
  "$userland_ucat_obj"

userland_init_obj="$build_dir/userland-init-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/init_clang.c -o "$userland_init_obj"
test -s "$userland_init_obj"
userland_init_dump="$build_dir/userland-init-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$userland_init_obj" \
  >"$userland_init_dump"
grep -q 'call ' "$userland_init_dump"
printf 'real LLVM LNP64 clang userland init object smoke passed: %s\n' \
  "$userland_init_obj"

userland_lnpsh_obj="$build_dir/userland-lnpsh-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/lnpsh_clang.c -o "$userland_lnpsh_obj"
test -s "$userland_lnpsh_obj"
userland_lnpsh_dump="$build_dir/userland-lnpsh-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$userland_lnpsh_obj" \
  >"$userland_lnpsh_dump"
grep -q 'call ' "$userland_lnpsh_dump"
printf 'real LLVM LNP64 clang userland lnpsh object smoke passed: %s\n' \
  "$userland_lnpsh_obj"

userland_spawn_obj="$build_dir/userland-spawn-task-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/spawn_task_clang.c -o "$userland_spawn_obj"
test -s "$userland_spawn_obj"
userland_spawn_dump="$build_dir/userland-spawn-task-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$userland_spawn_obj" \
  >"$userland_spawn_dump"
grep -q 'clone.spawn r' "$userland_spawn_dump"
grep -q 'thread_join r' "$userland_spawn_dump"
printf 'real LLVM LNP64 clang userland spawn task object smoke passed: %s\n' \
  "$userland_spawn_obj"

netbsd_init_obj="$build_dir/netbsd-init-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/netbsd_init_clang.c -o "$netbsd_init_obj"
test -s "$netbsd_init_obj"
netbsd_init_dump="$build_dir/netbsd-init-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_init_obj" \
  >"$netbsd_init_dump"
grep -q 'call ' "$netbsd_init_dump"
printf 'real LLVM LNP64 clang NetBSD init object passed: %s\n' \
  "$netbsd_init_obj"

netbsd_sh_obj="$build_dir/netbsd-sh-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/netbsd_sh_clang.c -o "$netbsd_sh_obj"
test -s "$netbsd_sh_obj"
netbsd_sh_dump="$build_dir/netbsd-sh-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_sh_obj" \
  >"$netbsd_sh_dump"
grep -q 'call ' "$netbsd_sh_dump"
grep -q 'domain_ctl r' "$netbsd_sh_dump"
printf 'real LLVM LNP64 clang NetBSD shell object passed: %s\n' \
  "$netbsd_sh_obj"

netbsd_loader_target_obj="$build_dir/netbsd-loader-target-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/loader_target_clang.c -o "$netbsd_loader_target_obj"
test -s "$netbsd_loader_target_obj"
netbsd_loader_target_dump="$build_dir/netbsd-loader-target-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_loader_target_obj" \
  >"$netbsd_loader_target_dump"
grep -q 'call ' "$netbsd_loader_target_dump"
printf 'real LLVM LNP64 clang NetBSD loader target child object passed: %s\n' \
  "$netbsd_loader_target_obj"

netbsd_elf_exec_test_obj="$build_dir/netbsd-elf-exec-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/elf_exec_test_clang.c -o "$netbsd_elf_exec_test_obj"
test -s "$netbsd_elf_exec_test_obj"
netbsd_elf_exec_test_dump="$build_dir/netbsd-elf-exec-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_elf_exec_test_obj" \
  >"$netbsd_elf_exec_test_dump"
grep -q 'call ' "$netbsd_elf_exec_test_dump"
printf 'real LLVM LNP64 clang NetBSD ELF exec parent object passed: %s\n' \
  "$netbsd_elf_exec_test_obj"

netbsd_fork_wait_test_obj="$build_dir/netbsd-fork-wait-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/fork_wait_test_clang.c -o "$netbsd_fork_wait_test_obj"
test -s "$netbsd_fork_wait_test_obj"
netbsd_fork_wait_test_dump="$build_dir/netbsd-fork-wait-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_fork_wait_test_obj" \
  >"$netbsd_fork_wait_test_dump"
grep -q 'call ' "$netbsd_fork_wait_test_dump"
printf 'real LLVM LNP64 clang NetBSD fork/wait child object passed: %s\n' \
  "$netbsd_fork_wait_test_obj"

netbsd_thread_test_obj="$build_dir/netbsd-thread-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/thread_test_clang.c -o "$netbsd_thread_test_obj"
test -s "$netbsd_thread_test_obj"
netbsd_thread_test_dump="$build_dir/netbsd-thread-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_thread_test_obj" \
  >"$netbsd_thread_test_dump"
grep -q 'call ' "$netbsd_thread_test_dump"
printf 'real LLVM LNP64 clang NetBSD thread child object passed: %s\n' \
  "$netbsd_thread_test_obj"

netbsd_poll_test_obj="$build_dir/netbsd-poll-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/poll_test_clang.c -o "$netbsd_poll_test_obj"
test -s "$netbsd_poll_test_obj"
netbsd_poll_test_dump="$build_dir/netbsd-poll-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_poll_test_obj" \
  >"$netbsd_poll_test_dump"
grep -q 'object_ctl r' "$netbsd_poll_test_dump"
grep -q 'push r' "$netbsd_poll_test_dump"
grep -q 'pull r' "$netbsd_poll_test_dump"
printf 'real LLVM LNP64 clang NetBSD poll child object passed: %s\n' \
  "$netbsd_poll_test_obj"

netbsd_signal_gate_test_obj="$build_dir/netbsd-signal-gate-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/signal_gate_test_clang.c -o "$netbsd_signal_gate_test_obj"
test -s "$netbsd_signal_gate_test_obj"
netbsd_signal_gate_test_dump="$build_dir/netbsd-signal-gate-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_signal_gate_test_obj" \
  >"$netbsd_signal_gate_test_dump"
grep -q 'call ' "$netbsd_signal_gate_test_dump"
grep -q 'yield' "$netbsd_signal_gate_test_dump"
printf 'real LLVM LNP64 clang NetBSD signal gate child object passed: %s\n' \
  "$netbsd_signal_gate_test_obj"

netbsd_signal_fault_test_obj="$build_dir/netbsd-signal-fault-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/signal_fault_test_clang.c -o "$netbsd_signal_fault_test_obj"
test -s "$netbsd_signal_fault_test_obj"
netbsd_signal_fault_test_dump="$build_dir/netbsd-signal-fault-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_signal_fault_test_obj" \
  >"$netbsd_signal_fault_test_dump"
grep -q 'div r' "$netbsd_signal_fault_test_dump"
grep -q 'sigret' "$netbsd_signal_fault_test_dump"
printf 'real LLVM LNP64 clang NetBSD signal fault child object passed: %s\n' \
  "$netbsd_signal_fault_test_obj"

netbsd_timer_test_obj="$build_dir/netbsd-timer-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/timer_test_clang.c -o "$netbsd_timer_test_obj"
test -s "$netbsd_timer_test_obj"
netbsd_timer_test_dump="$build_dir/netbsd-timer-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_timer_test_obj" \
  >"$netbsd_timer_test_dump"
grep -q 'call ' "$netbsd_timer_test_dump"
grep -q 'yield' "$netbsd_timer_test_dump"
grep -q 'sigret' "$netbsd_timer_test_dump"
printf 'real LLVM LNP64 clang NetBSD timer child object passed: %s\n' \
  "$netbsd_timer_test_obj"

netbsd_mmap_test_obj="$build_dir/netbsd-mmap-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/mmap_test_clang.c -o "$netbsd_mmap_test_obj"
test -s "$netbsd_mmap_test_obj"
netbsd_mmap_test_dump="$build_dir/netbsd-mmap-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_mmap_test_obj" \
  >"$netbsd_mmap_test_dump"
grep -q 'call ' "$netbsd_mmap_test_dump"
printf 'real LLVM LNP64 clang NetBSD mmap child object passed: %s\n' \
  "$netbsd_mmap_test_obj"

netbsd_fd_passing_test_obj="$build_dir/netbsd-fd-passing-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/fd_passing_test_clang.c -o "$netbsd_fd_passing_test_obj"
test -s "$netbsd_fd_passing_test_obj"
netbsd_fd_passing_test_dump="$build_dir/netbsd-fd-passing-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_fd_passing_test_obj" \
  >"$netbsd_fd_passing_test_dump"
grep -q 'object_ctl r' "$netbsd_fd_passing_test_dump"
grep -q 'cap_dup r' "$netbsd_fd_passing_test_dump"
grep -q 'cap_send r' "$netbsd_fd_passing_test_dump"
grep -q 'cap_recv r' "$netbsd_fd_passing_test_dump"
grep -q 'cap_revoke r' "$netbsd_fd_passing_test_dump"
grep -q 'errno_get r' "$netbsd_fd_passing_test_dump"
printf 'real LLVM LNP64 clang NetBSD fd passing child object passed: %s\n' \
  "$netbsd_fd_passing_test_obj"

netbsd_namespace_test_obj="$build_dir/netbsd-namespace-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/namespace_test_clang.c -o "$netbsd_namespace_test_obj"
test -s "$netbsd_namespace_test_obj"
netbsd_namespace_test_dump="$build_dir/netbsd-namespace-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_namespace_test_obj" \
  >"$netbsd_namespace_test_dump"
grep -q 'call ' "$netbsd_namespace_test_dump"
printf 'real LLVM LNP64 clang NetBSD namespace child object passed: %s\n' \
  "$netbsd_namespace_test_obj"

netbsd_fs_service_test_obj="$build_dir/netbsd-fs-service-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/fs_service_test_clang.c -o "$netbsd_fs_service_test_obj"
test -s "$netbsd_fs_service_test_obj"
netbsd_fs_service_test_dump="$build_dir/netbsd-fs-service-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_fs_service_test_obj" \
  >"$netbsd_fs_service_test_dump"
grep -q 'call ' "$netbsd_fs_service_test_dump"
grep -q 'ld.b r' "$netbsd_fs_service_test_dump"
grep -q 'st.b r' "$netbsd_fs_service_test_dump"
printf 'real LLVM LNP64 clang NetBSD fs service child object passed: %s\n' \
  "$netbsd_fs_service_test_obj"

netbsd_classifier_test_obj="$build_dir/netbsd-classifier-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/classifier_test_clang.c -o "$netbsd_classifier_test_obj"
test -s "$netbsd_classifier_test_obj"
netbsd_classifier_test_dump="$build_dir/netbsd-classifier-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_classifier_test_obj" \
  >"$netbsd_classifier_test_dump"
grep -q 'object_ctl r' "$netbsd_classifier_test_dump"
grep -q 'cap_dup r' "$netbsd_classifier_test_dump"
grep -q 'pull r' "$netbsd_classifier_test_dump"
printf 'real LLVM LNP64 clang NetBSD classifier child object passed: %s\n' \
  "$netbsd_classifier_test_obj"

netbsd_socket_loopback_test_obj="$build_dir/netbsd-socket-loopback-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/socket_loopback_test_clang.c -o "$netbsd_socket_loopback_test_obj"
test -s "$netbsd_socket_loopback_test_obj"
netbsd_socket_loopback_test_dump="$build_dir/netbsd-socket-loopback-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_socket_loopback_test_obj" \
  >"$netbsd_socket_loopback_test_dump"
grep -q 'call ' "$netbsd_socket_loopback_test_dump"
printf 'real LLVM LNP64 clang NetBSD socket loopback child object passed: %s\n' \
  "$netbsd_socket_loopback_test_obj"

# Exercises __lnp_domain_create and __lnp_call_gate_create record builders.
netbsd_gate_trace_test_obj="$build_dir/netbsd-gate-trace-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/gate_trace_test_clang.c -o "$netbsd_gate_trace_test_obj"
test -s "$netbsd_gate_trace_test_obj"
netbsd_gate_trace_test_dump="$build_dir/netbsd-gate-trace-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_gate_trace_test_obj" \
  >"$netbsd_gate_trace_test_dump"
grep -q 'domain_ctl r' "$netbsd_gate_trace_test_dump"
grep -q 'object_ctl r' "$netbsd_gate_trace_test_dump"
grep -q 'gate_call r' "$netbsd_gate_trace_test_dump"
grep -q 'gate_return r' "$netbsd_gate_trace_test_dump"
printf 'real LLVM LNP64 clang NetBSD gate trace child object passed: %s\n' \
  "$netbsd_gate_trace_test_obj"

netbsd_domain_nested_test_obj="$build_dir/netbsd-domain-nested-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/domain_nested_test_clang.c -o "$netbsd_domain_nested_test_obj"
test -s "$netbsd_domain_nested_test_obj"
netbsd_domain_nested_test_dump="$build_dir/netbsd-domain-nested-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_domain_nested_test_obj" \
  >"$netbsd_domain_nested_test_dump"
grep -q 'domain_ctl r' "$netbsd_domain_nested_test_dump"
printf 'real LLVM LNP64 clang NetBSD domain nested child object passed: %s\n' \
  "$netbsd_domain_nested_test_obj"

netbsd_domain_budget_test_obj="$build_dir/netbsd-domain-budget-test-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c userland/domain_budget_test_clang.c -o "$netbsd_domain_budget_test_obj"
test -s "$netbsd_domain_budget_test_obj"
netbsd_domain_budget_test_dump="$build_dir/netbsd-domain-budget-test-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_domain_budget_test_obj" \
  >"$netbsd_domain_budget_test_dump"
grep -q 'domain_ctl r' "$netbsd_domain_budget_test_dump"
grep -q 'alloc r' "$netbsd_domain_budget_test_dump"
printf 'real LLVM LNP64 clang NetBSD domain budget child object passed: %s\n' \
  "$netbsd_domain_budget_test_obj"

meta_libc_c="$build_dir/meta-libc-smoke.c"
cat >"$meta_libc_c" <<'C'
#include <errno.h>
#include <dirent.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

int main(void) {
  struct stat st;
  char cwd[256];
  char linkbuf[32];
  DIR *dir;
  struct timespec omit[2] = {
      {.tv_nsec = UTIME_OMIT},
      {.tv_nsec = UTIME_OMIT},
  };
  unlink("target/llvm-lnp64-build/meta_ns_link");
  unlink("target/llvm-lnp64-build/meta_ns_hard");
  unlink("target/llvm-lnp64-build/meta_ns_dir/renamed");
  unlink("target/llvm-lnp64-build/meta_ns_dir/file");
  rmdir("target/llvm-lnp64-build/meta_ns_dir");
  if (!getcwd(cwd, sizeof(cwd)))
    return 10;
  dir = opendir("target/llvm-lnp64-build");
  if (!dir)
    return 11;
  if (closedir(dir) != 0)
    return 12;
  int fd = open("Cargo.toml", O_RDONLY);
  if (fd < 0)
    return 1;
  if (stat("Cargo.toml", &st) != 0)
    return 2;
  if (!S_ISREG(st.st_mode))
    return 3;
  if (st.st_size <= 0)
    return 4;
  if (st.st_nlink <= 0)
    return 5;
  if (fstat(fd, &st) != 0)
    return 6;
  if (fcntl(fd, F_GETFD) < 0)
    return 7;
  errno = 0;
  if (futimens(-1, omit) != -1)
    return 8;
  if (errno != EBADF)
    return 9;
  close(fd);
  if (faccessat(AT_FDCWD, "Cargo.toml", F_OK, 0) != 0)
    return 13;
  if (mkdirat(AT_FDCWD, "target/llvm-lnp64-build/meta_ns_dir", 0755) != 0)
    return 14;
  fd = open("target/llvm-lnp64-build/meta_ns_dir/file", O_CREAT | O_TRUNC | O_RDWR);
  if (fd < 0)
    return 15;
  close(fd);
  if (fchmodat(AT_FDCWD, "target/llvm-lnp64-build/meta_ns_dir/file", 0600, 0) != 0)
    return 16;
  if (fchownat(AT_FDCWD, "target/llvm-lnp64-build/meta_ns_dir/file",
               (uid_t)-1, (gid_t)-1, 0) != 0)
    return 17;
  if (symlinkat("meta_ns_dir/file", AT_FDCWD,
                "target/llvm-lnp64-build/meta_ns_link") != 0)
    return 18;
  if (readlinkat(AT_FDCWD, "target/llvm-lnp64-build/meta_ns_link",
                 linkbuf, sizeof(linkbuf)) != 16)
    return 19;
  if (linkbuf[0] != 'm' || linkbuf[12] != 'f')
    return 20;
  if (linkat(AT_FDCWD, "target/llvm-lnp64-build/meta_ns_dir/file",
             AT_FDCWD, "target/llvm-lnp64-build/meta_ns_hard", 0) != 0)
    return 21;
  if (renameat(AT_FDCWD, "target/llvm-lnp64-build/meta_ns_dir/file",
               AT_FDCWD, "target/llvm-lnp64-build/meta_ns_dir/renamed") != 0)
    return 22;
  if (stat("target/llvm-lnp64-build/meta_ns_dir/renamed", &st) != 0)
    return 23;
  if (unlink("target/llvm-lnp64-build/meta_ns_link") != 0)
    return 24;
  if (unlink("target/llvm-lnp64-build/meta_ns_hard") != 0)
    return 25;
  if (unlink("target/llvm-lnp64-build/meta_ns_dir/renamed") != 0)
    return 26;
  if (rmdir("target/llvm-lnp64-build/meta_ns_dir") != 0)
    return 27;
  return 0;
}
C

meta_libc_obj="$build_dir/meta-libc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c "$meta_libc_c" -o "$meta_libc_obj"
test -s "$meta_libc_obj"
meta_libc_dump="$build_dir/meta-libc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$meta_libc_obj" \
  >"$meta_libc_dump"
grep -q 'call ' "$meta_libc_dump"
printf 'real LLVM LNP64 clang metadata libc object smoke passed: %s\n' \
  "$meta_libc_obj"

mmap_libc_c="$build_dir/mmap-libc-smoke.c"
cat >"$mmap_libc_c" <<'C'
typedef unsigned long size_t;

void *mmap(void *addr, size_t len, int prot, int flags, int fd, long offset);
int mprotect(void *addr, size_t len, int prot);
int munmap(void *addr, size_t len);

enum {
  MAP_PRIVATE = 0x02,
  MAP_ANONYMOUS = 0x20,
};

int main(void) {
  unsigned char *bytes =
      mmap(0, 4096, 3, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (bytes == (void *)~0UL)
    return 1;
  bytes[0] = 0x5a;
  bytes[7] = 0xa5;
  if (bytes[0] != 0x5a || bytes[7] != 0xa5)
    return 2;
  if (mprotect(bytes, 4096, 1) != 0)
    return 3;
  if (munmap(bytes, 4096) != 0)
    return 4;
  return 0;
}
C

mmap_libc_obj="$build_dir/mmap-libc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$mmap_libc_c" -o "$mmap_libc_obj"
test -s "$mmap_libc_obj"
mmap_libc_dump="$build_dir/mmap-libc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$mmap_libc_obj" \
  >"$mmap_libc_dump"
grep -q 'call ' "$mmap_libc_dump"
printf 'real LLVM LNP64 clang mmap libc object smoke passed: %s\n' \
  "$mmap_libc_obj"

futex_libc_c="$build_dir/futex-libc-smoke.c"
cat >"$futex_libc_c" <<'C'
typedef unsigned long lnp64_word_t;

int futex_wait(volatile lnp64_word_t *addr, lnp64_word_t expected);
int futex_wake(volatile lnp64_word_t *addr, lnp64_word_t count);

static volatile lnp64_word_t futex_cell = 1;

int main(void) {
  if (futex_wait(&futex_cell, 0) != 0)
    return 1;
  return futex_wake(&futex_cell, 1);
}
C

futex_libc_obj="$build_dir/futex-libc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$futex_libc_c" -o "$futex_libc_obj"
test -s "$futex_libc_obj"
futex_libc_dump="$build_dir/futex-libc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$futex_libc_obj" \
  >"$futex_libc_dump"
grep -q 'call ' "$futex_libc_dump"
printf 'real LLVM LNP64 clang futex libc object smoke passed: %s\n' \
  "$futex_libc_obj"

poll_libc_c="$build_dir/poll-libc-smoke.c"
cat >"$poll_libc_c" <<'C'
typedef unsigned long nfds_t;

typedef struct {
  unsigned long bits[16];
} fd_set;

struct timeval {
  long tv_sec;
  long tv_usec;
};

struct timespec {
  long tv_sec;
  long tv_nsec;
};

struct pollfd {
  int fd;
  short events;
  short revents;
};

struct epoll_event {
  unsigned int events;
  unsigned long data;
};

struct kevent {
  unsigned long ident;
  short filter;
  unsigned short flags;
  unsigned int fflags;
  long data;
  void *udata;
};

int poll(struct pollfd *fds, nfds_t nfds, int timeout);
int select(int nfds, fd_set *readfds, fd_set *writefds, fd_set *exceptfds,
           struct timeval *timeout);
int epoll_create1(int flags);
int epoll_ctl(int epfd, int op, int fd, struct epoll_event *event);
int epoll_wait(int epfd, struct epoll_event *events, int maxevents,
               int timeout);
int kqueue(void);
int kevent(int kq, const struct kevent *changelist, int nchanges,
           struct kevent *eventlist, int nevents, const struct timespec *timeout);

int main(void) {
  struct pollfd fds[2];
  struct epoll_event ev;
  struct epoll_event out;
  struct kevent change;
  struct timespec ts = {0, 0};
  fd_set readfds;
  fd_set writefds;
  fd_set exceptfds;
  struct timeval timeout = {0, 0};
  int ep;
  int kq;
  readfds.bits[0] = 0;
  writefds.bits[0] = 0;
  exceptfds.bits[0] = 0;
  fds[0].fd = 0;
  fds[0].events = 0;
  fds[0].revents = 7;
  fds[1].fd = -1;
  fds[1].events = 1;
  fds[1].revents = 9;
  if (poll(fds, 2, 0) != 0)
    return 1;
  if (fds[0].revents != 0)
    return 2;
  if (fds[1].revents != 0)
    return 3;
  if (select(1, &readfds, &writefds, &exceptfds, &timeout) != 0)
    return 4;
  ep = epoll_create1(0);
  if (ep < 0)
    return 5;
  ev.events = 0;
  ev.data = 42;
  if (epoll_ctl(ep, 1, 0, &ev) != 0)
    return 6;
  if (epoll_wait(ep, &out, 1, 0) != 0)
    return 7;
  kq = kqueue();
  if (kq < 0)
    return 8;
  change.ident = 0;
  change.filter = -1;
  change.flags = 1;
  change.fflags = 0;
  change.data = 0;
  change.udata = 0;
  if (kevent(kq, &change, 1, 0, 0, &ts) != 0)
    return 9;
  return 0;
}
C

poll_libc_obj="$build_dir/poll-libc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$poll_libc_c" -o "$poll_libc_obj"
test -s "$poll_libc_obj"
poll_libc_dump="$build_dir/poll-libc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$poll_libc_obj" \
  >"$poll_libc_dump"
grep -q 'call ' "$poll_libc_dump"
printf 'real LLVM LNP64 clang poll/select/epoll/kqueue libc object smoke passed: %s\n' \
  "$poll_libc_obj"

signal_libc_c="$build_dir/signal-libc-smoke.c"
cat >"$signal_libc_c" <<'C'
#include "lnp64_intrinsics.h"

typedef unsigned long sigset_t;
typedef void (*sighandler_t)(int);

struct sigaction {
  sighandler_t sa_handler;
  sigset_t sa_mask;
  int sa_flags;
};

#define SIG_IGN ((sighandler_t)1)

sighandler_t signal(int signum, sighandler_t handler);
int sigaction(int signum, const struct sigaction *act,
              struct sigaction *oldact);
int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);
int kill(int pid, int signum);
int raise(int signum);
unsigned int alarm(unsigned int seconds);

int main(void) {
  struct sigaction act;
  sigset_t mask = 0;
  act.sa_handler = SIG_IGN;
  act.sa_mask = 0;
  act.sa_flags = 0;
  if (signal(10, SIG_IGN) != 0)
    return 1;
  if (sigaction(12, &act, 0) != 0)
    return 2;
  if (sigprocmask(2, &mask, 0) != 0)
    return 3;
  if (kill((int)__lnp_get_pid(), 10) != 0)
    return 4;
  if (raise(12) != 0)
    return 5;
  if (alarm(0) != 0)
    return 6;
  return 0;
}
C

signal_libc_obj="$build_dir/signal-libc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$signal_libc_c" -o "$signal_libc_obj"
test -s "$signal_libc_obj"
signal_libc_dump="$build_dir/signal-libc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$signal_libc_obj" \
  >"$signal_libc_dump"
grep -q 'call ' "$signal_libc_dump"
printf 'real LLVM LNP64 clang signal libc object smoke passed: %s\n' \
  "$signal_libc_obj"

socket_libc_c="$build_dir/socket-libc-smoke.c"
cat >"$socket_libc_c" <<'C'
typedef unsigned long size_t;
typedef unsigned long socklen_t;
enum {
  AF_INET = 2,
  SOCK_STREAM = 1,
  SOL_SOCKET = 1,
  SO_REUSEADDR = 2,
  SO_ERROR = 4,
  MSG_NOSIGNAL = 0x4000
};

int socket(int domain, int type, int protocol);
int bind(int fd, const void *addr, socklen_t len);
int listen(int fd, int backlog);
int connect(int fd, const void *addr, socklen_t len);
int accept(int fd, void *addr, socklen_t *len);
int getsockname(int fd, void *addr, socklen_t *len);
int getsockopt(int fd, int level, int optname, void *optval, socklen_t *optlen);
int setsockopt(int fd, int level, int optname, const void *optval,
               socklen_t optlen);
long send(int fd, const void *buf, size_t len, int flags);
long recv(int fd, void *buf, size_t len, int flags);

int main(void) {
  int server;
  int client;
  int accepted;
  int i;
  long got;
  int opt = 1;
  int so_error = 99;
  socklen_t optlen = 8;
  char addr[64];
  socklen_t addrlen = 64;
  char buf[2];

  server = socket(AF_INET, SOCK_STREAM, 0);
  if (server < 0)
    return 1;
  if (setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &opt, 8) != 0)
    return 2;
  if (getsockopt(server, SOL_SOCKET, SO_ERROR, &so_error, &optlen) != 0)
    return 3;
  if (so_error != 0 || optlen != 8)
    return 4;
  if (bind(server, "127.0.0.1:0", 0) != 0)
    return 5;
  if (listen(server, 1) != 0)
    return 6;
  if (getsockname(server, addr, &addrlen) != 0)
    return 7;
  client = socket(AF_INET, SOCK_STREAM, 0);
  if (client < 0)
    return 8;
  if (connect(client, addr, addrlen) != 0)
    return 9;
  accepted = -1;
  for (i = 0; i < 1000; i = i + 1) {
    accepted = accept(server, 0, 0);
    if (accepted >= 0)
      break;
  }
  if (accepted < 0)
    return 10;
  if (send(client, "z", 1, MSG_NOSIGNAL) != 1)
    return 11;
  got = -1;
  for (i = 0; i < 1000; i = i + 1) {
    got = recv(accepted, buf, 1, 0);
    if (got == 1)
      break;
  }
  if (got != 1)
    return 12;
  if (buf[0] != 'z')
    return 13;
  return 0;
}
C

socket_libc_obj="$build_dir/socket-libc-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$socket_libc_c" -o "$socket_libc_obj"
test -s "$socket_libc_obj"
socket_libc_dump="$build_dir/socket-libc-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$socket_libc_obj" \
  >"$socket_libc_dump"
grep -q 'call ' "$socket_libc_dump"
printf 'real LLVM LNP64 clang socket libc object smoke passed: %s\n' \
  "$socket_libc_obj"

netbsd_personality_clang_c="userland/netbsd_personality_clang_smoke.c"
netbsd_personality_clang_obj="$build_dir/netbsd-personality-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c "$netbsd_personality_clang_c" -o "$netbsd_personality_clang_obj"
test -s "$netbsd_personality_clang_obj"
netbsd_personality_clang_dump="$build_dir/netbsd-personality-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none \
  "$netbsd_personality_clang_obj" >"$netbsd_personality_clang_dump"
grep -q 'call ' "$netbsd_personality_clang_dump"
printf 'real LLVM LNP64 clang NetBSD personality smoke object passed: %s\n' \
  "$netbsd_personality_clang_obj"

libc_fd_impl_c="toolchain/liblnp64_fd_min.c"
libc_fd_impl_obj="$build_dir/liblnp64-fd-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_fd_impl_c" -o "$libc_fd_impl_obj"
test -s "$libc_fd_impl_obj"
libc_fd_impl_dump="$build_dir/liblnp64-fd-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_fd_impl_obj" \
  >"$libc_fd_impl_dump"
grep -q 'pull r' "$libc_fd_impl_dump"
grep -q 'push r' "$libc_fd_impl_dump"
grep -q 'fd_seek_dyn r' "$libc_fd_impl_dump"
grep -q 'cap_revoke r' "$libc_fd_impl_dump"
grep -q 'ret' "$libc_fd_impl_dump"
printf 'real LLVM LNP64 clang minilibc fd implementation object smoke passed: %s\n' \
  "$libc_fd_impl_obj"

libc_meta_impl_c="toolchain/liblnp64_meta_min.c"
libc_meta_impl_obj="$build_dir/liblnp64-meta-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -I toolchain/include \
  -c "$libc_meta_impl_c" -o "$libc_meta_impl_obj"
test -s "$libc_meta_impl_obj"
libc_meta_impl_dump="$build_dir/liblnp64-meta-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_meta_impl_obj" \
  >"$libc_meta_impl_dump"
grep -q 'stat_path_at r' "$libc_meta_impl_dump"
grep -q 'stat_fd_dyn r' "$libc_meta_impl_dump"
grep -q 'utime_path_at r' "$libc_meta_impl_dump"
grep -q 'utime_fd_dyn r' "$libc_meta_impl_dump"
grep -q 'fcntl_fd_dyn r' "$libc_meta_impl_dump"
grep -q 'open_dir_dyn r' "$libc_meta_impl_dump"
grep -q 'mkdir_path_at r' "$libc_meta_impl_dump"
grep -q 'rename_path_at r' "$libc_meta_impl_dump"
grep -q 'link_path_at r' "$libc_meta_impl_dump"
grep -q 'symlink_path_at r' "$libc_meta_impl_dump"
grep -q 'readlink_path_at r' "$libc_meta_impl_dump"
grep -q 'chdir_path r' "$libc_meta_impl_dump"
grep -q 'getcwd_path r' "$libc_meta_impl_dump"
grep -q 'chmod_path_at r' "$libc_meta_impl_dump"
grep -q 'chown_path_at r' "$libc_meta_impl_dump"
grep -q 'errno_get r' "$libc_meta_impl_dump"
printf 'real LLVM LNP64 clang minilibc metadata implementation object smoke passed: %s\n' \
  "$libc_meta_impl_obj"

libc_poll_impl_c="toolchain/liblnp64_poll_min.c"
libc_poll_impl_obj="$build_dir/liblnp64-poll-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_poll_impl_c" -o "$libc_poll_impl_obj"
test -s "$libc_poll_impl_obj"
libc_poll_impl_dump="$build_dir/liblnp64-poll-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_poll_impl_obj" \
  >"$libc_poll_impl_dump"
grep -q 'await r' "$libc_poll_impl_dump"
grep -q 'ret' "$libc_poll_impl_dump"
printf 'real LLVM LNP64 clang minilibc poll/select/epoll/kqueue implementation object smoke passed: %s\n' \
  "$libc_poll_impl_obj"

libc_signal_impl_c="toolchain/liblnp64_signal_min.c"
libc_signal_impl_obj="$build_dir/liblnp64-signal-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_signal_impl_c" -o "$libc_signal_impl_obj"
test -s "$libc_signal_impl_obj"
libc_signal_impl_dump="$build_dir/liblnp64-signal-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_signal_impl_obj" \
  >"$libc_signal_impl_dump"
grep -q 'sigaction r' "$libc_signal_impl_dump"
grep -q 'sigmask_set r' "$libc_signal_impl_dump"
grep -q 'kill r' "$libc_signal_impl_dump"
grep -q 'alarm r' "$libc_signal_impl_dump"
grep -q 'ret' "$libc_signal_impl_dump"
printf 'real LLVM LNP64 clang minilibc signal implementation object smoke passed: %s\n' \
  "$libc_signal_impl_obj"

libc_socket_impl_c="toolchain/liblnp64_socket_min.c"
libc_socket_impl_obj="$build_dir/liblnp64-socket-min.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-builtin -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$libc_socket_impl_c" -o "$libc_socket_impl_obj"
test -s "$libc_socket_impl_obj"
libc_socket_impl_dump="$build_dir/liblnp64-socket-min.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$libc_socket_impl_obj" \
  >"$libc_socket_impl_dump"
grep -q 'object_ctl r' "$libc_socket_impl_dump"
grep -q 'push r' "$libc_socket_impl_dump"
grep -q 'pull r' "$libc_socket_impl_dump"
grep -q 'ret' "$libc_socket_impl_dump"
printf 'real LLVM LNP64 clang minilibc socket implementation object smoke passed: %s\n' \
  "$libc_socket_impl_obj"

stack_args_c="$build_dir/stack-args-smoke.c"
cat >"$stack_args_c" <<'C'
__attribute__((noinline)) int sum7(int a, int b, int c, int d, int e, int f, int g) {
  return a + b + c + d + e + f + g;
}
int main(void) {
  return sum7(1, 2, 3, 4, 5, 6, 7) - 28;
}
C

stack_args_obj="$build_dir/stack-args-clang-smoke.o"
"$clang" --target=lnp64-unknown-none -ffreestanding -fno-pic -fno-jump-tables \
  -fno-unwind-tables -fno-asynchronous-unwind-tables -I toolchain \
  -c "$stack_args_c" -o "$stack_args_obj"
test -s "$stack_args_obj"
stack_args_dump="$build_dir/stack-args-clang-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$stack_args_obj" \
  >"$stack_args_dump"
grep -q 'call ' "$stack_args_dump"
grep -q 'lr_get r' "$stack_args_dump"
grep -q 'lr_set r' "$stack_args_dump"
grep -q 'st ' "$stack_args_dump"
grep -q 'ld ' "$stack_args_dump"
printf 'real LLVM LNP64 clang stack-argument object smoke passed: %s\n' \
  "$stack_args_obj"

if [[ "$gate" == "objects" ]]; then
  printf 'real LLVM LNP64 object-only gate passed: %s\n' "$build_dir"
  exit 0
fi

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

auipc_asm="$build_dir/auipc-mc-smoke.s"
cat >"$auipc_asm" <<'ASM'
  .text
  .globl _start
_start:
  auipc r1, 4096
  auipc r2, target
  ret
target:
  nop
ASM
auipc_mc_obj="$build_dir/auipc-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$auipc_asm" \
  -o "$auipc_mc_obj"
test -s "$auipc_mc_obj"
auipc_mc_dump="$build_dir/auipc-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$auipc_mc_obj" \
  >"$auipc_mc_dump"
grep -q 'auipc r1, 4096' "$auipc_mc_dump"
grep -q 'auipc r2' "$auipc_mc_dump"
printf 'real LLVM LNP64 llvm-mc auipc smoke passed: %s\n' "$auipc_mc_obj"

mmap_asm="$build_dir/mmap-mc-smoke.s"
cat >"$mmap_asm" <<'ASM'
  .text
  .globl _start
_start:
  mmap r1, r2, r3, r4
  munmap r5, r6
  mprotect r7, r8, r9, r10
  ret
ASM
mmap_mc_obj="$build_dir/mmap-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$mmap_asm" \
  -o "$mmap_mc_obj"
test -s "$mmap_mc_obj"
mmap_mc_dump="$build_dir/mmap-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$mmap_mc_obj" \
  >"$mmap_mc_dump"
grep -q 'mmap r1, r2, r3, r4' "$mmap_mc_dump"
grep -q 'munmap r5, r6' "$mmap_mc_dump"
grep -q 'mprotect r7, r8, r9, r10' "$mmap_mc_dump"
printf 'real LLVM LNP64 llvm-mc mmap opcode smoke passed: %s\n' "$mmap_mc_obj"

env_get_asm="$build_dir/env-get-mc-smoke.s"
cat >"$env_get_asm" <<'ASM'
  .text
  .globl _start
_start:
  env_get r1, r2, r3, r4
  ret
ASM
env_get_mc_obj="$build_dir/env-get-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$env_get_asm" \
  -o "$env_get_mc_obj"
test -s "$env_get_mc_obj"
env_get_mc_dump="$build_dir/env-get-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$env_get_mc_obj" \
  >"$env_get_mc_dump"
grep -q 'env_get r1, r2, r3, r4' "$env_get_mc_dump"
printf 'real LLVM LNP64 llvm-mc env_get opcode smoke passed: %s\n' \
  "$env_get_mc_obj"

get_pcr_asm="$build_dir/get-pcr-mc-smoke.s"
cat >"$get_pcr_asm" <<'ASM'
  .text
  .globl _start
_start:
  get_pcr r1, PID
  set_pcr r3, SIGMASK, r2
  get_pcr r4, CRED_PROFILE
  set_pcr r5, CRED_HANDLE, r2
  ret
ASM
get_pcr_mc_obj="$build_dir/get-pcr-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$get_pcr_asm" \
  -o "$get_pcr_mc_obj"
test -s "$get_pcr_mc_obj"
get_pcr_mc_dump="$build_dir/get-pcr-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$get_pcr_mc_obj" \
  >"$get_pcr_mc_dump"
grep -q 'get_pcr r1, PID' "$get_pcr_mc_dump"
grep -q 'set_pcr r3, SIGMASK, r2' "$get_pcr_mc_dump"
grep -q 'get_pcr r4, CRED_PROFILE' "$get_pcr_mc_dump"
grep -q 'set_pcr r5, CRED_HANDLE, r2' "$get_pcr_mc_dump"
stale_set_pcr_asm="$build_dir/stale-set-pcr-mc-smoke.s"
cat >"$stale_set_pcr_asm" <<'ASM'
  .text
  .globl _start
_start:
  set_pcr TP, r2
  ret
ASM
stale_set_pcr_err="$build_dir/stale-set-pcr-mc-smoke.err"
if "$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$stale_set_pcr_asm" \
  -o "$build_dir/stale-set-pcr-mc-smoke.o" 2>"$stale_set_pcr_err"; then
  printf 'stale two-operand SET_PCR unexpectedly assembled\n' >&2
  exit 1
fi
printf 'real LLVM LNP64 llvm-mc GET_PCR opcode smoke passed: %s\n' \
  "$get_pcr_mc_obj"

open_at_asm="$build_dir/open-at-mc-smoke.s"
cat >"$open_at_asm" <<'ASM'
  .text
  .globl _start
_start:
  open_at r1, r2, r3, r4
ASM
open_at_mc_obj="$build_dir/open-at-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$open_at_asm" \
  -o "$open_at_mc_obj"
test -s "$open_at_mc_obj"
open_at_mc_dump="$build_dir/open-at-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$open_at_mc_obj" \
  >"$open_at_mc_dump"
grep -q 'open_at r1, r2, r3, r4' "$open_at_mc_dump"
printf 'real LLVM LNP64 llvm-mc OPEN_AT opcode smoke passed: %s\n' \
  "$open_at_mc_obj"

clone_control_asm="$build_dir/clone-control-mc-smoke.s"
cat >"$clone_control_asm" <<'ASM'
  .text
  .globl _start
_start:
  clone.spawn r1, r2, r3
  thread_join r4, r5, r6
ASM
clone_control_mc_obj="$build_dir/clone-control-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$clone_control_asm" \
  -o "$clone_control_mc_obj"
test -s "$clone_control_mc_obj"
clone_control_mc_dump="$build_dir/clone-control-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$clone_control_mc_obj" \
  >"$clone_control_mc_dump"
grep -q 'clone.spawn r1, r2, r3' "$clone_control_mc_dump"
grep -q 'thread_join r4, r5, r6' "$clone_control_mc_dump"
printf 'real LLVM LNP64 llvm-mc clone control opcode smoke passed: %s\n' \
  "$clone_control_mc_obj"

compat_meta_asm="$build_dir/compat-meta-mc-smoke.s"
cat >"$compat_meta_asm" <<'ASM'
  .text
  .globl _start
_start:
  stat_path_at r1, r2, r3, r4
  stat_fd_dyn r5, r6
  utime_path_at r7, r8, r9, r10
  utime_fd_dyn r11, r12
  fcntl_fd_dyn r13, r14, r15
  fd_seek_dyn r16, r17, r18
  unlink_path_at r19, r20, r21
  open_dir_dyn r22, r23, r24
  mkdir_path_at r25, r26, r27
  rename_path_at r1, r2, r3, r4
  link_path_at r5, r6, r7, r8, r9
  symlink_path_at r10, r11, r12
  readlink_path_at r13, r14, r15, r16
  chdir_path r17
  getcwd_path r18, r19
  chmod_path_at r20, r21, r22, r23
  chown_path_at r24, r25, r26, r27, r28
  ret
ASM
compat_meta_mc_obj="$build_dir/compat-meta-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$compat_meta_asm" \
  -o "$compat_meta_mc_obj"
test -s "$compat_meta_mc_obj"
compat_meta_mc_dump="$build_dir/compat-meta-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$compat_meta_mc_obj" \
  >"$compat_meta_mc_dump"
grep -q 'stat_path_at r1, r2, r3, r4' "$compat_meta_mc_dump"
grep -q 'stat_fd_dyn r5, r6' "$compat_meta_mc_dump"
grep -q 'utime_path_at r7, r8, r9, r10' "$compat_meta_mc_dump"
grep -q 'utime_fd_dyn r11, r12' "$compat_meta_mc_dump"
grep -q 'fcntl_fd_dyn r13, r14, r15' "$compat_meta_mc_dump"
grep -q 'fd_seek_dyn r16, r17, r18' "$compat_meta_mc_dump"
grep -q 'unlink_path_at r19, r20, r21' "$compat_meta_mc_dump"
grep -q 'open_dir_dyn r22, r23, r24' "$compat_meta_mc_dump"
grep -q 'mkdir_path_at r25, r26, r27' "$compat_meta_mc_dump"
grep -q 'rename_path_at r1, r2, r3, r4' "$compat_meta_mc_dump"
grep -q 'link_path_at r5, r6, r7, r8, r9' "$compat_meta_mc_dump"
grep -q 'symlink_path_at r10, r11, r12' "$compat_meta_mc_dump"
grep -q 'readlink_path_at r13, r14, r15, r16' "$compat_meta_mc_dump"
grep -q 'chdir_path r17' "$compat_meta_mc_dump"
grep -q 'getcwd_path r18, r19' "$compat_meta_mc_dump"
grep -q 'chmod_path_at r20, r21, r22, r23' "$compat_meta_mc_dump"
grep -q 'chown_path_at r24, r25, r26, r27, r28' "$compat_meta_mc_dump"
printf 'real LLVM LNP64 llvm-mc compatibility metadata opcode smoke passed: %s\n' \
  "$compat_meta_mc_obj"

cap_control_asm="$build_dir/cap-control-mc-smoke.s"
cat >"$cap_control_asm" <<'ASM'
  .text
  .globl _start
_start:
  cap_dup r1, r2
  cap_send r3, r4
  cap_recv r5, r6
  cap_revoke r7, r8
  ret
ASM
cap_control_mc_obj="$build_dir/cap-control-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$cap_control_asm" \
  -o "$cap_control_mc_obj"
test -s "$cap_control_mc_obj"
cap_control_mc_dump="$build_dir/cap-control-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$cap_control_mc_obj" \
  >"$cap_control_mc_dump"
grep -q 'cap_dup r1, r2' "$cap_control_mc_dump"
grep -q 'cap_send r3, r4' "$cap_control_mc_dump"
grep -q 'cap_recv r5, r6' "$cap_control_mc_dump"
grep -q 'cap_revoke r7, r8' "$cap_control_mc_dump"
printf 'real LLVM LNP64 llvm-mc capability control opcode smoke passed: %s\n' \
  "$cap_control_mc_obj"

atomic_asm="$build_dir/atomic-mc-smoke.s"
cat >"$atomic_asm" <<'ASM'
  .text
  .globl _start
_start:
  amo.swap r1, r2, r3
  amo.add r4, r5, r6
  amo.and r7, r8, r9
  amo.or r10, r11, r12
  lock.cmpxchg r13, r14, r15, r16
  amo.xor r17, r18, r19
  futex_wait r20, r21
  futex_wake r22, r23
  fence
  fence.acq
  fence.rel
  fence.acq_rel
  fence.sc
  isync r24, r25, r26
  ret
ASM
atomic_mc_obj="$build_dir/atomic-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$atomic_asm" \
  -o "$atomic_mc_obj"
test -s "$atomic_mc_obj"
atomic_mc_dump="$build_dir/atomic-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$atomic_mc_obj" \
  >"$atomic_mc_dump"
grep -q 'amo.swap r1, r2, r3' "$atomic_mc_dump"
grep -q 'amo.add r4, r5, r6' "$atomic_mc_dump"
grep -q 'amo.and r7, r8, r9' "$atomic_mc_dump"
grep -q 'amo.or r10, r11, r12' "$atomic_mc_dump"
grep -q 'lock.cmpxchg r13, r14, r15, r16' "$atomic_mc_dump"
grep -q 'amo.xor r17, r18, r19' "$atomic_mc_dump"
grep -q 'futex_wait r20, r21' "$atomic_mc_dump"
grep -q 'futex_wake r22, r23' "$atomic_mc_dump"
grep -q 'fence' "$atomic_mc_dump"
grep -q 'isync r24, r25, r26' "$atomic_mc_dump"
printf 'real LLVM LNP64 llvm-mc atomic opcode smoke passed: %s\n' \
  "$atomic_mc_obj"

signal_alias_asm="$build_dir/signal-alias-mc-smoke.s"
cat >"$signal_alias_asm" <<'ASM'
  .text
  .globl _start
_start:
  sigaction r1, r2
  sigmask_set r3
  kill r4, r5
  alarm r6, r7
  yield
  sigret
ASM
signal_alias_mc_obj="$build_dir/signal-alias-mc-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj "$signal_alias_asm" \
  -o "$signal_alias_mc_obj"
test -s "$signal_alias_mc_obj"
signal_alias_mc_dump="$build_dir/signal-alias-mc-smoke.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$signal_alias_mc_obj" \
  >"$signal_alias_mc_dump"
grep -q 'sigaction r1, r2' "$signal_alias_mc_dump"
grep -q 'sigmask_set r3' "$signal_alias_mc_dump"
grep -q 'kill r4, r5' "$signal_alias_mc_dump"
grep -q 'alarm r6, r7' "$signal_alias_mc_dump"
grep -q 'yield' "$signal_alias_mc_dump"
grep -q 'sigret' "$signal_alias_mc_dump"
printf 'real LLVM LNP64 llvm-mc signal alias opcode smoke passed: %s\n' \
  "$signal_alias_mc_obj"

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

intrinsic_cap_elf="$build_dir/lnp64-intrinsic-cap-control-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_cap_elf" "$crt0_obj" "$intrinsic_cap_obj"
test -s "$intrinsic_cap_elf"
printf 'real LLVM LNP64 lld intrinsic capability control link smoke passed: %s\n' \
  "$intrinsic_cap_elf"

intrinsic_mmap_elf="$build_dir/lnp64-intrinsic-mmap-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_mmap_elf" "$crt0_obj" "$intrinsic_mmap_obj"
test -s "$intrinsic_mmap_elf"
printf 'real LLVM LNP64 lld intrinsic mmap link smoke passed: %s\n' \
  "$intrinsic_mmap_elf"

intrinsic_get_pcr_elf="$build_dir/lnp64-intrinsic-get-pcr-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_get_pcr_elf" "$crt0_obj" "$intrinsic_get_pcr_obj"
test -s "$intrinsic_get_pcr_elf"
printf 'real LLVM LNP64 lld intrinsic GET_PCR link smoke passed: %s\n' \
  "$intrinsic_get_pcr_elf"

intrinsic_set_pcr_elf="$build_dir/lnp64-intrinsic-set-pcr-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_set_pcr_elf" "$crt0_obj" "$intrinsic_set_pcr_obj"
test -s "$intrinsic_set_pcr_elf"
printf 'real LLVM LNP64 lld intrinsic SET_PCR link smoke passed: %s\n' \
  "$intrinsic_set_pcr_elf"

intrinsic_openat_elf="$build_dir/lnp64-intrinsic-openat-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_openat_elf" "$crt0_obj" "$intrinsic_openat_obj"
test -s "$intrinsic_openat_elf"
printf 'real LLVM LNP64 lld intrinsic OPEN_AT link smoke passed: %s\n' \
  "$intrinsic_openat_elf"

intrinsic_clone_elf="$build_dir/lnp64-intrinsic-clone-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_clone_elf" "$crt0_obj" "$intrinsic_clone_obj"
test -s "$intrinsic_clone_elf"
printf 'real LLVM LNP64 lld intrinsic CLONE link smoke passed: %s\n' \
  "$intrinsic_clone_elf"

intrinsic_amo_elf="$build_dir/lnp64-intrinsic-amo-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$intrinsic_amo_elf" "$crt0_obj" "$intrinsic_amo_obj"
test -s "$intrinsic_amo_elf"
printf 'real LLVM LNP64 lld intrinsic AMO link smoke passed: %s\n' \
  "$intrinsic_amo_elf"

c11_atomic_elf="$build_dir/lnp64-c11-atomic-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$c11_atomic_elf" "$crt0_obj" "$c11_atomic_obj"
test -s "$c11_atomic_elf"
printf 'real LLVM LNP64 lld C11 atomic link smoke passed: %s\n' \
  "$c11_atomic_elf"

inline_asm_elf="$build_dir/lnp64-inline-asm-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$inline_asm_elf" "$crt0_obj" "$inline_asm_obj"
test -s "$inline_asm_elf"
printf 'real LLVM LNP64 lld inline asm link smoke passed: %s\n' \
  "$inline_asm_elf"

exit_elf="$build_dir/lnp64-exit-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$exit_elf" "$crt0_obj" "$exit_obj" "$libc_process_impl_obj"
test -s "$exit_elf"
printf 'real LLVM LNP64 lld exit link smoke passed: %s\n' "$exit_elf"

errno_elf="$build_dir/lnp64-errno-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$errno_elf" "$crt0_obj" "$errno_obj" "$libc_errno_impl_obj"
test -s "$errno_elf"
printf 'real LLVM LNP64 lld errno link smoke passed: %s\n' "$errno_elf"

argc_elf="$build_dir/lnp64-argc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$argc_elf" "$crt0_obj" "$argc_obj"
test -s "$argc_elf"
printf 'real LLVM LNP64 lld argc link smoke passed: %s\n' "$argc_elf"

startup_elf="$build_dir/lnp64-startup-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$startup_elf" "$crt0_obj" "$startup_obj"
test -s "$startup_elf"
printf 'real LLVM LNP64 lld startup argv/envp link smoke passed: %s\n' \
  "$startup_elf"

getauxval_elf="$build_dir/lnp64-getauxval-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$getauxval_elf" "$crt0_obj" "$getauxval_obj" "$libc_startup_impl_obj" \
  "$libc_errno_impl_obj"
test -s "$getauxval_elf"
printf 'real LLVM LNP64 lld getauxval link smoke passed: %s\n' \
  "$getauxval_elf"

libc_test_argv_elf="$build_dir/lnp64-libc-test-argv-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_argv_elf" "$crt0_obj" "$libc_test_argv_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_argv_elf"
printf 'real LLVM LNP64 lld libc-test argv link smoke passed: %s\n' \
  "$libc_test_argv_elf"

libc_test_env_elf="$build_dir/lnp64-libc-test-env-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_env_elf" "$crt0_obj" "$libc_test_env_obj" \
  "$libc_test_print_obj" "$libc_startup_impl_obj" "$libc_string_impl_obj" \
  "$libc_errno_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_env_elf"
printf 'real LLVM LNP64 lld libc-test env link smoke passed: %s\n' \
  "$libc_test_env_elf"

libc_test_random_elf="$build_dir/lnp64-libc-test-random-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_random_elf" "$crt0_obj" "$libc_test_random_obj" \
  "$libc_test_print_obj" "$libc_random_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_random_elf"
printf 'real LLVM LNP64 lld libc-test random link smoke passed: %s\n' \
  "$libc_test_random_elf"

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

stack_args_elf="$build_dir/lnp64-stack-args-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$stack_args_elf" "$crt0_obj" "$stack_args_obj"
test -s "$stack_args_elf"
printf 'real LLVM LNP64 lld stack-argument link smoke passed: %s\n' \
  "$stack_args_elf"

libc_string_elf="$build_dir/lnp64-libc-string-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_string_elf" "$crt0_obj" "$libc_string_obj" "$libc_string_impl_obj"
test -s "$libc_string_elf"
printf 'real LLVM LNP64 lld minilibc string link smoke passed: %s\n' \
  "$libc_string_elf"

convert_elf="$build_dir/lnp64-convert-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$convert_elf" "$crt0_obj" "$convert_obj" "$libc_convert_impl_obj" \
  "$libc_errno_impl_obj"
test -s "$convert_elf"
printf 'real LLVM LNP64 lld numeric conversion link smoke passed: %s\n' \
  "$convert_elf"

path_elf="$build_dir/lnp64-path-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$path_elf" "$crt0_obj" "$path_obj" "$libc_path_impl_obj" \
  "$libc_string_impl_obj"
test -s "$path_elf"
printf 'real LLVM LNP64 lld path helper link smoke passed: %s\n' \
  "$path_elf"

search_elf="$build_dir/lnp64-search-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$search_elf" "$crt0_obj" "$search_obj" "$libc_search_impl_obj" \
  "$libc_string_impl_obj"
test -s "$search_elf"
printf 'real LLVM LNP64 lld search helper link smoke passed: %s\n' \
  "$search_elf"

sort_elf="$build_dir/lnp64-sort-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$sort_elf" "$crt0_obj" "$sort_obj" "$libc_sort_impl_obj" \
  "$libc_string_impl_obj"
test -s "$sort_elf"
printf 'real LLVM LNP64 lld sort helper link smoke passed: %s\n' \
  "$sort_elf"

zlib_elf="$build_dir/lnp64-zlib-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$zlib_elf" "$crt0_obj" "$zlib_smoke_obj" "$zlib_adler_obj" \
  "$zlib_crc_obj" "$libc_string_impl_obj"
test -s "$zlib_elf"
printf 'real LLVM LNP64 lld zlib package link smoke passed: %s\n' \
  "$zlib_elf"

natsort_elf="$build_dir/lnp64-natsort-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$natsort_elf" "$crt0_obj" "$natsort_smoke_obj" "$natsort_impl_obj" \
  "$libc_string_impl_obj"
test -s "$natsort_elf"
printf 'real LLVM LNP64 lld natsort package link smoke passed: %s\n' \
  "$natsort_elf"

jsmn_elf="$build_dir/lnp64-jsmn-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$jsmn_elf" "$crt0_obj" "$jsmn_smoke_obj"
test -s "$jsmn_elf"
printf 'real LLVM LNP64 lld jsmn package link smoke passed: %s\n' \
  "$jsmn_elf"

inih_elf="$build_dir/lnp64-inih-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$inih_elf" "$crt0_obj" "$inih_smoke_obj" "$libc_string_impl_obj"
test -s "$inih_elf"
printf 'real LLVM LNP64 lld inih package link smoke passed: %s\n' \
  "$inih_elf"

cwalk_elf="$build_dir/lnp64-cwalk-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$cwalk_elf" "$crt0_obj" "$cwalk_smoke_obj" "$cwalk_impl_obj" \
  "$libc_string_impl_obj"
test -s "$cwalk_elf"
printf 'real LLVM LNP64 lld cwalk package link smoke passed: %s\n' \
  "$cwalk_elf"

libc_test_ctype_elf="$build_dir/lnp64-libc-test-ctype-bounded-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_ctype_elf" "$crt0_obj" "$libc_test_ctype_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_ctype_elf"
printf 'real LLVM LNP64 lld libc-test ctype_bounded link smoke passed: %s\n' \
  "$libc_test_ctype_elf"

libc_test_string_elf="$build_dir/lnp64-libc-test-string-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_string_elf" "$crt0_obj" "$libc_test_string_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_string_elf"
printf 'real LLVM LNP64 lld libc-test string link smoke passed: %s\n' \
  "$libc_test_string_elf"

libc_test_memcpy_bounded_elf="$build_dir/lnp64-libc-test-string-memcpy-bounded-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_memcpy_bounded_elf" "$crt0_obj" "$libc_test_memcpy_bounded_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_memcpy_bounded_elf"
printf 'real LLVM LNP64 lld libc-test string_memcpy_bounded link smoke passed: %s\n' \
  "$libc_test_memcpy_bounded_elf"

libc_test_memmove_bounded_elf="$build_dir/lnp64-libc-test-string-memmove-bounded-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_memmove_bounded_elf" "$crt0_obj" "$libc_test_memmove_bounded_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_memmove_bounded_elf"
printf 'real LLVM LNP64 lld libc-test string_memmove_bounded link smoke passed: %s\n' \
  "$libc_test_memmove_bounded_elf"

libc_test_memmem_elf="$build_dir/lnp64-libc-test-string-memmem-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_memmem_elf" "$crt0_obj" "$libc_test_memmem_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_memmem_elf"
printf 'real LLVM LNP64 lld libc-test string_memmem link smoke passed: %s\n' \
  "$libc_test_memmem_elf"

libc_test_strchr_elf="$build_dir/lnp64-libc-test-string-strchr-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_strchr_elf" "$crt0_obj" "$libc_test_strchr_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_strchr_elf"
printf 'real LLVM LNP64 lld libc-test string_strchr link smoke passed: %s\n' \
  "$libc_test_strchr_elf"

libc_test_strcspn_elf="$build_dir/lnp64-libc-test-string-strcspn-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_strcspn_elf" "$crt0_obj" "$libc_test_strcspn_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_strcspn_elf"
printf 'real LLVM LNP64 lld libc-test string_strcspn link smoke passed: %s\n' \
  "$libc_test_strcspn_elf"

libc_test_strstr_elf="$build_dir/lnp64-libc-test-string-strstr-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_strstr_elf" "$crt0_obj" "$libc_test_strstr_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_strstr_elf"
printf 'real LLVM LNP64 lld libc-test string_strstr link smoke passed: %s\n' \
  "$libc_test_strstr_elf"

libc_test_udiv_elf="$build_dir/lnp64-libc-test-udiv-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_udiv_elf" "$crt0_obj" "$libc_test_udiv_obj" \
  "$libc_test_print_obj" "$libc_fd_impl_obj"
test -s "$libc_test_udiv_elf"
printf 'real LLVM LNP64 lld libc-test udiv link smoke passed: %s\n' \
  "$libc_test_udiv_elf"

libc_test_basename_elf="$build_dir/lnp64-libc-test-basename-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_basename_elf" "$crt0_obj" "$libc_test_basename_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_path_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$libc_test_basename_elf"
printf 'real LLVM LNP64 lld libc-test basename link smoke passed: %s\n' \
  "$libc_test_basename_elf"

libc_test_dirname_elf="$build_dir/lnp64-libc-test-dirname-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_dirname_elf" "$crt0_obj" "$libc_test_dirname_obj" \
  "$libc_test_print_obj" "$libc_string_impl_obj" "$libc_path_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$libc_test_dirname_elf"
printf 'real LLVM LNP64 lld libc-test dirname link smoke passed: %s\n' \
  "$libc_test_dirname_elf"

libc_test_strtol_elf="$build_dir/lnp64-libc-test-strtol-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_strtol_elf" "$crt0_obj" "$libc_test_strtol_obj" \
  "$libc_test_print_obj" "$libc_convert_impl_obj" "$libc_errno_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$libc_test_strtol_elf"
printf 'real LLVM LNP64 lld libc-test strtol link smoke passed: %s\n' \
  "$libc_test_strtol_elf"

libc_test_clock_gettime_elf="$build_dir/lnp64-libc-test-clock-gettime-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_clock_gettime_elf" "$crt0_obj" "$libc_test_clock_gettime_obj" \
  "$libc_test_print_obj" "$libc_time_impl_obj" "$libc_errno_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$libc_test_clock_gettime_elf"
printf 'real LLVM LNP64 lld libc-test clock_gettime link smoke passed: %s\n' \
  "$libc_test_clock_gettime_elf"

libc_test_access_bounded_elf="$build_dir/lnp64-libc-test-access-bounded-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_access_bounded_elf" "$crt0_obj" \
  "$libc_test_access_bounded_obj" "$libc_test_print_obj" \
  "$libc_meta_impl_obj" "$libc_fd_impl_obj" "$libc_errno_impl_obj"
test -s "$libc_test_access_bounded_elf"
printf 'real LLVM LNP64 lld libc-test access_bounded link smoke passed: %s\n' \
  "$libc_test_access_bounded_elf"

libc_test_stat_elf="$build_dir/lnp64-libc-test-stat-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_stat_elf" "$crt0_obj" "$libc_test_stat_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj" "$libc_time_impl_obj" \
  "$libc_process_impl_obj"
test -s "$libc_test_stat_elf"
printf 'real LLVM LNP64 lld libc-test stat link smoke passed: %s\n' \
  "$libc_test_stat_elf"

libc_test_utime_elf="$build_dir/lnp64-libc-test-utime-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_utime_elf" "$crt0_obj" "$libc_test_utime_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj" "$libc_time_impl_obj" \
  "$libc_process_impl_obj" "$libc_string_impl_obj"
test -s "$libc_test_utime_elf"
printf 'real LLVM LNP64 lld libc-test utime link smoke passed: %s\n' \
  "$libc_test_utime_elf"

libc_test_ungetc_elf="$build_dir/lnp64-libc-test-ungetc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_ungetc_elf" "$crt0_obj" "$libc_test_ungetc_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj"
test -s "$libc_test_ungetc_elf"
printf 'real LLVM LNP64 lld libc-test ungetc link smoke passed: %s\n' \
  "$libc_test_ungetc_elf"

libc_test_fdopen_elf="$build_dir/lnp64-libc-test-fdopen-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_fdopen_elf" "$crt0_obj" "$libc_test_fdopen_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj"
test -s "$libc_test_fdopen_elf"
printf 'real LLVM LNP64 lld libc-test fdopen link smoke passed: %s\n' \
  "$libc_test_fdopen_elf"

libc_test_fcntl_basic_elf="$build_dir/lnp64-libc-test-fcntl-basic-bounded-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_fcntl_basic_elf" "$crt0_obj" \
  "$libc_test_fcntl_basic_obj" "$libc_test_print_obj" \
  "$libc_stdio_impl_obj" "$libc_meta_impl_obj" "$libc_fd_impl_obj" \
  "$libc_errno_impl_obj"
test -s "$libc_test_fcntl_basic_elf"
printf 'real LLVM LNP64 lld libc-test fcntl_basic_bounded link smoke passed: %s\n' \
  "$libc_test_fcntl_basic_elf"

libc_test_pthread_tsd_elf="$build_dir/lnp64-libc-test-pthread-tsd-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_pthread_tsd_elf" "$crt0_obj" \
  "$libc_test_pthread_tsd_obj" "$libc_test_print_obj" \
  "$libc_pthread_impl_obj" "$libc_alloc_impl_obj" "$libc_string_impl_obj" \
  "$libc_errno_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_pthread_tsd_elf"
printf 'real LLVM LNP64 lld libc-test pthread_tsd link smoke passed: %s\n' \
  "$libc_test_pthread_tsd_elf"

libc_test_sem_init_elf="$build_dir/lnp64-libc-test-sem-init-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_sem_init_elf" "$crt0_obj" \
  "$libc_test_sem_init_obj" "$libc_test_print_obj" \
  "$libc_pthread_impl_obj" "$libc_sem_impl_obj" "$libc_futex_impl_obj" \
  "$libc_alloc_impl_obj" "$libc_string_impl_obj" "$libc_errno_impl_obj" \
  "$libc_time_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_sem_init_elf"
printf 'real LLVM LNP64 lld libc-test sem_init link smoke passed: %s\n' \
  "$libc_test_sem_init_elf"

libc_test_qsort_bounded_elf="$build_dir/lnp64-libc-test-qsort-bounded-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_qsort_bounded_elf" "$crt0_obj" "$libc_test_qsort_bounded_obj" \
  "$libc_test_print_obj" "$libc_sort_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$libc_test_qsort_bounded_elf"
printf 'real LLVM LNP64 lld libc-test qsort_bounded link smoke passed: %s\n' \
  "$libc_test_qsort_bounded_elf"

libc_test_search_insque_elf="$build_dir/lnp64-libc-test-search-insque-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_search_insque_elf" "$crt0_obj" "$libc_test_search_insque_obj" \
  "$libc_test_print_obj" "$libc_search_impl_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_search_insque_elf"
printf 'real LLVM LNP64 lld libc-test search_insque link smoke passed: %s\n' \
  "$libc_test_search_insque_elf"

libc_test_search_lsearch_elf="$build_dir/lnp64-libc-test-search-lsearch-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_search_lsearch_elf" "$crt0_obj" \
  "$libc_test_search_lsearch_obj" "$libc_test_print_obj" \
  "$libc_search_impl_obj" "$libc_string_impl_obj" "$libc_fd_impl_obj"
test -s "$libc_test_search_lsearch_elf"
printf 'real LLVM LNP64 lld libc-test search_lsearch link smoke passed: %s\n' \
  "$libc_test_search_lsearch_elf"

libc_test_malloc_0_elf="$build_dir/lnp64-libc-test-malloc-0-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_malloc_0_elf" "$crt0_obj" "$libc_test_malloc_0_obj" \
  "$libc_test_print_obj" "$libc_alloc_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$libc_test_malloc_0_elf"
printf 'real LLVM LNP64 lld libc-test malloc-0 link smoke passed: %s\n' \
  "$libc_test_malloc_0_elf"

libc_test_fgets_eof_elf="$build_dir/lnp64-libc-test-fgets-eof-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$libc_test_fgets_eof_elf" "$crt0_obj" "$libc_test_fgets_eof_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$libc_test_fgets_eof_elf"
printf 'real LLVM LNP64 lld libc-test fgets-eof link smoke passed: %s\n' \
  "$libc_test_fgets_eof_elf"

calloc_elf="$build_dir/lnp64-calloc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$calloc_elf" "$crt0_obj" "$calloc_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj"
test -s "$calloc_elf"
printf 'real LLVM LNP64 lld calloc link smoke passed: %s\n' \
  "$calloc_elf"

realloc_elf="$build_dir/lnp64-realloc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$realloc_elf" "$crt0_obj" "$realloc_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj"
test -s "$realloc_elf"
printf 'real LLVM LNP64 lld realloc link smoke passed: %s\n' \
  "$realloc_elf"

read_elf="$build_dir/lnp64-read-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$read_elf" "$crt0_obj" "$read_obj" "$libc_fd_impl_obj"
test -s "$read_elf"
printf 'real LLVM LNP64 lld read link smoke passed: %s\n' \
  "$read_elf"

write_elf="$build_dir/lnp64-write-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$write_elf" "$crt0_obj" "$write_obj" "$libc_fd_impl_obj"
test -s "$write_elf"
printf 'real LLVM LNP64 lld write link smoke passed: %s\n' \
  "$write_elf"

userland_ucat_elf="$build_dir/lnp64-userland-ucat-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$userland_ucat_elf" "$crt0_obj" "$userland_ucat_obj" \
  "$libc_fd_impl_obj"
test -s "$userland_ucat_elf"
printf 'real LLVM LNP64 lld userland ucat link smoke passed: %s\n' \
  "$userland_ucat_elf"

userland_init_elf="$build_dir/lnp64-userland-init-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$userland_init_elf" "$crt0_obj" "$userland_init_obj" \
  "$libc_fd_impl_obj"
test -s "$userland_init_elf"
printf 'real LLVM LNP64 lld userland init link smoke passed: %s\n' \
  "$userland_init_elf"

userland_lnpsh_elf="$build_dir/lnp64-userland-lnpsh-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$userland_lnpsh_elf" "$crt0_obj" "$userland_lnpsh_obj" \
  "$libc_fd_impl_obj"
test -s "$userland_lnpsh_elf"
printf 'real LLVM LNP64 lld userland lnpsh link smoke passed: %s\n' \
  "$userland_lnpsh_elf"

userland_spawn_elf="$build_dir/lnp64-userland-spawn-task-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$userland_spawn_elf" "$crt0_obj" "$userland_spawn_obj" \
  "$libc_fd_impl_obj"
test -s "$userland_spawn_elf"
printf 'real LLVM LNP64 lld userland spawn task link smoke passed: %s\n' \
  "$userland_spawn_elf"

netbsd_init_elf="$build_dir/lnp64-netbsd-init-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_init_elf" "$crt0_obj" "$netbsd_init_obj" \
  "$libc_process_impl_obj" "$libc_errno_impl_obj" "$libc_fd_impl_obj" \
  "$libc_startup_impl_obj"
test -s "$netbsd_init_elf"
printf 'real LLVM LNP64 lld NetBSD init link passed: %s\n' \
  "$netbsd_init_elf"

netbsd_sh_elf="$build_dir/lnp64-netbsd-sh-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_sh_elf" "$crt0_obj" "$netbsd_sh_obj" \
  "$libc_process_impl_obj" "$libc_errno_impl_obj" "$libc_fd_impl_obj" \
  "$libc_meta_impl_obj" "$libc_startup_impl_obj"
test -s "$netbsd_sh_elf"
printf 'real LLVM LNP64 lld NetBSD shell link passed: %s\n' \
  "$netbsd_sh_elf"

netbsd_loader_target_elf="$build_dir/lnp64-netbsd-loader-target-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_loader_target_elf" "$crt0_obj" "$netbsd_loader_target_obj" \
  "$libc_fd_impl_obj"
test -s "$netbsd_loader_target_elf"
printf 'real LLVM LNP64 lld NetBSD loader target child link passed: %s\n' \
  "$netbsd_loader_target_elf"

netbsd_elf_exec_test_elf="$build_dir/lnp64-netbsd-elf-exec-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_elf_exec_test_elf" "$crt0_obj" "$netbsd_elf_exec_test_obj" \
  "$libc_process_impl_obj" "$libc_errno_impl_obj" "$libc_fd_impl_obj" \
  "$libc_startup_impl_obj"
test -s "$netbsd_elf_exec_test_elf"
printf 'real LLVM LNP64 lld NetBSD ELF exec parent link passed: %s\n' \
  "$netbsd_elf_exec_test_elf"

netbsd_fork_wait_test_elf="$build_dir/lnp64-netbsd-fork-wait-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_fork_wait_test_elf" "$crt0_obj" "$netbsd_fork_wait_test_obj" \
  "$libc_process_impl_obj" "$libc_errno_impl_obj" "$libc_fd_impl_obj"
test -s "$netbsd_fork_wait_test_elf"
printf 'real LLVM LNP64 lld NetBSD fork/wait child link passed: %s\n' \
  "$netbsd_fork_wait_test_elf"

netbsd_thread_test_elf="$build_dir/lnp64-netbsd-thread-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_thread_test_elf" "$crt0_obj" "$netbsd_thread_test_obj" \
  "$libc_pthread_impl_obj" "$libc_alloc_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj"
test -s "$netbsd_thread_test_elf"
printf 'real LLVM LNP64 lld NetBSD thread child link passed: %s\n' \
  "$netbsd_thread_test_elf"

netbsd_poll_test_elf="$build_dir/lnp64-netbsd-poll-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_poll_test_elf" "$crt0_obj" "$netbsd_poll_test_obj" \
  "$libc_poll_impl_obj" "$libc_fd_impl_obj"
test -s "$netbsd_poll_test_elf"
printf 'real LLVM LNP64 lld NetBSD poll child link passed: %s\n' \
  "$netbsd_poll_test_elf"

netbsd_signal_gate_test_elf="$build_dir/lnp64-netbsd-signal-gate-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_signal_gate_test_elf" "$crt0_obj" "$netbsd_signal_gate_test_obj" \
  "$libc_signal_impl_obj" "$libc_fd_impl_obj"
test -s "$netbsd_signal_gate_test_elf"
printf 'real LLVM LNP64 lld NetBSD signal gate child link passed: %s\n' \
  "$netbsd_signal_gate_test_elf"

netbsd_signal_fault_test_elf="$build_dir/lnp64-netbsd-signal-fault-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_signal_fault_test_elf" "$crt0_obj" "$netbsd_signal_fault_test_obj" \
  "$libc_signal_impl_obj" "$libc_fd_impl_obj"
test -s "$netbsd_signal_fault_test_elf"
printf 'real LLVM LNP64 lld NetBSD signal fault child link passed: %s\n' \
  "$netbsd_signal_fault_test_elf"

netbsd_timer_test_elf="$build_dir/lnp64-netbsd-timer-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_timer_test_elf" "$crt0_obj" "$netbsd_timer_test_obj" \
  "$libc_time_impl_obj" "$libc_signal_impl_obj" "$libc_poll_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj"
test -s "$netbsd_timer_test_elf"
printf 'real LLVM LNP64 lld NetBSD timer child link passed: %s\n' \
  "$netbsd_timer_test_elf"

netbsd_mmap_test_elf="$build_dir/lnp64-netbsd-mmap-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_mmap_test_elf" "$crt0_obj" "$netbsd_mmap_test_obj" \
  "$libc_vma_impl_obj" "$libc_errno_impl_obj" "$libc_fd_impl_obj"
test -s "$netbsd_mmap_test_elf"
printf 'real LLVM LNP64 lld NetBSD mmap child link passed: %s\n' \
  "$netbsd_mmap_test_elf"

netbsd_fd_passing_test_elf="$build_dir/lnp64-netbsd-fd-passing-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_fd_passing_test_elf" "$crt0_obj" "$netbsd_fd_passing_test_obj" \
  "$libc_fd_impl_obj"
test -s "$netbsd_fd_passing_test_elf"
printf 'real LLVM LNP64 lld NetBSD fd passing child link passed: %s\n' \
  "$netbsd_fd_passing_test_elf"

netbsd_namespace_test_elf="$build_dir/lnp64-netbsd-namespace-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_namespace_test_elf" "$crt0_obj" "$netbsd_namespace_test_obj" \
  "$libc_fd_impl_obj" "$libc_meta_impl_obj" "$libc_errno_impl_obj"
test -s "$netbsd_namespace_test_elf"
printf 'real LLVM LNP64 lld NetBSD namespace child link passed: %s\n' \
  "$netbsd_namespace_test_elf"

netbsd_fs_service_test_elf="$build_dir/lnp64-netbsd-fs-service-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_fs_service_test_elf" "$crt0_obj" "$netbsd_fs_service_test_obj" \
  "$libc_fd_impl_obj" "$libc_alloc_impl_obj" "$libc_string_impl_obj"
test -s "$netbsd_fs_service_test_elf"
printf 'real LLVM LNP64 lld NetBSD fs service child link passed: %s\n' \
  "$netbsd_fs_service_test_elf"

netbsd_classifier_test_elf="$build_dir/lnp64-netbsd-classifier-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_classifier_test_elf" "$crt0_obj" "$netbsd_classifier_test_obj" \
  "$libc_poll_impl_obj" "$libc_fd_impl_obj"
test -s "$netbsd_classifier_test_elf"
netbsd_classifier_test_linked_dump="$build_dir/netbsd-classifier-test-linked.dump"
"$llvm_objdump" -d --triple=lnp64-unknown-none "$netbsd_classifier_test_elf" \
  >"$netbsd_classifier_test_linked_dump"
grep -q 'await r' "$netbsd_classifier_test_linked_dump"
printf 'real LLVM LNP64 lld NetBSD classifier child link passed: %s\n' \
  "$netbsd_classifier_test_elf"

netbsd_socket_loopback_test_elf="$build_dir/lnp64-netbsd-socket-loopback-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_socket_loopback_test_elf" "$crt0_obj" \
  "$netbsd_socket_loopback_test_obj" "$libc_socket_impl_obj" \
  "$libc_poll_impl_obj" "$libc_fd_impl_obj" "$libc_errno_impl_obj"
test -s "$netbsd_socket_loopback_test_elf"
printf 'real LLVM LNP64 lld NetBSD socket loopback child link passed: %s\n' \
  "$netbsd_socket_loopback_test_elf"

netbsd_gate_trace_test_elf="$build_dir/lnp64-netbsd-gate-trace-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_gate_trace_test_elf" "$crt0_obj" "$netbsd_gate_trace_test_obj" \
  "$libc_fd_impl_obj"
test -s "$netbsd_gate_trace_test_elf"
printf 'real LLVM LNP64 lld NetBSD gate trace child link passed: %s\n' \
  "$netbsd_gate_trace_test_elf"

netbsd_domain_nested_test_elf="$build_dir/lnp64-netbsd-domain-nested-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_domain_nested_test_elf" "$crt0_obj" \
  "$netbsd_domain_nested_test_obj" "$libc_fd_impl_obj"
test -s "$netbsd_domain_nested_test_elf"
printf 'real LLVM LNP64 lld NetBSD domain nested child link passed: %s\n' \
  "$netbsd_domain_nested_test_elf"

netbsd_domain_budget_test_elf="$build_dir/lnp64-netbsd-domain-budget-test-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_domain_budget_test_elf" "$crt0_obj" \
  "$netbsd_domain_budget_test_obj" "$libc_fd_impl_obj"
test -s "$netbsd_domain_budget_test_elf"
printf 'real LLVM LNP64 lld NetBSD domain budget child link passed: %s\n' \
  "$netbsd_domain_budget_test_elf"

meta_libc_elf="$build_dir/lnp64-meta-libc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$meta_libc_elf" "$crt0_obj" "$meta_libc_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj"
test -s "$meta_libc_elf"
printf 'real LLVM LNP64 lld metadata libc link smoke passed: %s\n' \
  "$meta_libc_elf"

mmap_libc_elf="$build_dir/lnp64-mmap-libc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$mmap_libc_elf" "$crt0_obj" "$mmap_libc_obj" "$libc_vma_impl_obj" \
  "$libc_errno_impl_obj"
test -s "$mmap_libc_elf"
printf 'real LLVM LNP64 lld mmap libc link smoke passed: %s\n' \
  "$mmap_libc_elf"

futex_libc_elf="$build_dir/lnp64-futex-libc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$futex_libc_elf" "$crt0_obj" "$futex_libc_obj" "$libc_futex_impl_obj"
test -s "$futex_libc_elf"
printf 'real LLVM LNP64 lld futex libc link smoke passed: %s\n' \
  "$futex_libc_elf"

poll_libc_elf="$build_dir/lnp64-poll-libc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$poll_libc_elf" "$crt0_obj" "$poll_libc_obj" "$libc_poll_impl_obj"
test -s "$poll_libc_elf"
printf 'real LLVM LNP64 lld poll/select/epoll/kqueue libc link smoke passed: %s\n' \
  "$poll_libc_elf"

signal_libc_elf="$build_dir/lnp64-signal-libc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$signal_libc_elf" "$crt0_obj" "$signal_libc_obj" \
  "$libc_signal_impl_obj"
test -s "$signal_libc_elf"
printf 'real LLVM LNP64 lld signal libc link smoke passed: %s\n' \
  "$signal_libc_elf"

socket_libc_elf="$build_dir/lnp64-socket-libc-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$socket_libc_elf" "$crt0_obj" "$socket_libc_obj" \
  "$libc_socket_impl_obj" "$libc_errno_impl_obj"
test -s "$socket_libc_elf"
printf 'real LLVM LNP64 lld socket libc link smoke passed: %s\n' \
  "$socket_libc_elf"

netbsd_personality_clang_elf="$build_dir/lnp64-netbsd-personality-clang-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netbsd_personality_clang_elf" "$crt0_obj" \
  "$netbsd_personality_clang_obj" "$libc_fd_impl_obj" \
  "$libc_vma_impl_obj" "$libc_poll_impl_obj" "$libc_signal_impl_obj" \
  "$libc_socket_impl_obj" "$libc_pthread_impl_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj" "$libc_errno_impl_obj"
test -s "$netbsd_personality_clang_elf"
printf 'real LLVM LNP64 lld NetBSD personality clang smoke link passed: %s\n' \
  "$netbsd_personality_clang_elf"

sbase_echo_elf="$build_dir/lnp64-sbase-echo-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$sbase_echo_elf" "$crt0_obj" "$build_dir/sbase-echo-clang-smoke.o" \
  "$sbase_support_impl_obj" "$libc_fd_impl_obj" "$libc_string_impl_obj" \
  "$libc_process_impl_obj"
test -s "$sbase_echo_elf"
printf 'real LLVM LNP64 lld sbase echo link smoke passed: %s\n' \
  "$sbase_echo_elf"

for sbase_path_cmd in basename dirname; do
  sbase_path_elf="$build_dir/lnp64-sbase-$sbase_path_cmd-linked.elf"
  "$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
    -o "$sbase_path_elf" "$crt0_obj" \
    "$build_dir/sbase-$sbase_path_cmd-clang-smoke.o" \
    "$sbase_support_impl_obj" "$libc_fd_impl_obj" "$libc_string_impl_obj" \
    "$libc_path_impl_obj" "$libc_process_impl_obj"
  test -s "$sbase_path_elf"
done
printf 'real LLVM LNP64 lld sbase path command link smoke passed: %s %s\n' \
  "$build_dir/lnp64-sbase-basename-linked.elf" \
  "$build_dir/lnp64-sbase-dirname-linked.elf"

sbase_cat_elf="$build_dir/lnp64-sbase-cat-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$sbase_cat_elf" "$crt0_obj" "$build_dir/sbase-cat-clang-smoke.o" \
  "$build_dir/sbase-libutil-concat-clang-smoke.o" \
  "$build_dir/sbase-libutil-writeall-clang-smoke.o" \
  "$sbase_support_impl_obj" "$libc_fd_impl_obj" "$libc_string_impl_obj" \
  "$libc_process_impl_obj"
test -s "$sbase_cat_elf"
printf 'real LLVM LNP64 lld sbase cat link smoke passed: %s\n' \
  "$sbase_cat_elf"

netcat_elf="$build_dir/lnp64-netcat-clang-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$netcat_elf" "$crt0_obj" "$netcat_obj" "$libc_fd_impl_obj" \
  "$libc_alloc_impl_obj" "$libc_string_impl_obj" "$libc_poll_impl_obj" \
  "$libc_socket_impl_obj" "$libc_errno_impl_obj"
test -s "$netcat_elf"
printf 'real LLVM LNP64 lld netcat demo link smoke passed: %s\n' \
  "$netcat_elf"

httpd_elf="$build_dir/lnp64-httpd-clang-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$httpd_elf" "$crt0_obj" "$httpd_obj" "$libc_fd_impl_obj" \
  "$libc_alloc_impl_obj" "$libc_string_impl_obj" "$libc_poll_impl_obj" \
  "$libc_socket_impl_obj" "$libc_errno_impl_obj"
test -s "$httpd_elf"
printf 'real LLVM LNP64 lld httpd demo link smoke passed: %s\n' \
  "$httpd_elf"

indirect_call_elf="$build_dir/lnp64-indirect-call-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
  -o "$indirect_call_elf" "$crt0_obj" "$indirect_call_obj"
test -s "$indirect_call_elf"
printf 'real LLVM LNP64 lld indirect call link smoke passed: %s\n' \
  "$indirect_call_elf"

for demo in hello factorial allocator fibonacci pcr cat json-parser rot13 producer-consumer parallel-hash sqlite-lite ping-pong; do
  demo_obj="$build_dir/$demo-clang-smoke.o"
  demo_elf="$build_dir/lnp64-$demo-clang-linked.elf"
  "$lld" -flavor gnu -static -m elf64lnp64 -T toolchain/lnp64_static.ld \
    -o "$demo_elf" "$crt0_obj" "$demo_obj" "$libc_fd_impl_obj" \
    "$libc_alloc_impl_obj" "$libc_string_impl_obj" "$libc_process_impl_obj" \
    "$libc_futex_impl_obj"
  test -s "$demo_elf"
done
printf 'real LLVM LNP64 lld clang demo link smoke passed: %s\n' \
  "$build_dir/lnp64-hello-clang-linked.elf"

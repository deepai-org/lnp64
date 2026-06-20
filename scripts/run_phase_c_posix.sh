#!/usr/bin/env bash
set -euo pipefail

# Phase C: NetBSD/POSIX personality closure gate.
# Validates fork/waitpid, signals, timers, mmap, and socket loopback
# (bind/listen/accept/connect/poll/send/recv) under real Clang/lld/run-elf.

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
work_dir="${LNP64_POSIX_BUILD_DIR:-target/lnp64-posix-build}"
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

compile_flags=(
  --target=lnp64-unknown-none
  -ffreestanding -fno-pic
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables
  -isystem "$sysroot/usr/include"
  -I "$root/toolchain"
  -O0
)

libc_objs=(
  "$lib_dir/liblnp64-socket-min.o"
  "$lib_dir/liblnp64-stdio-min.o"
  "$lib_dir/liblnp64-alloc-min.o"
  "$lib_dir/liblnp64-string-min.o"
  "$lib_dir/liblnp64-convert-min.o"
  "$lib_dir/liblnp64-startup-min.o"
  "$lib_dir/liblnp64-signal-min.o"
  "$lib_dir/liblnp64-fd-min.o"
  "$lib_dir/liblnp64-errno-min.o"
  "$lib_dir/liblnp64-time-min.o"
  "$lib_dir/liblnp64-poll-min.o"
  "$lib_dir/liblnp64-process-min.o"
  "$lib_dir/liblnp64-meta-min.o"
  "$lib_dir/liblnp64-vma-min.o"
  "$lib_dir/liblnp64-softfloat-min.o"
)

build_and_run() {
  local name="$1"
  local src="$2"
  local expected_output="$3"

  local obj="$work_dir/${name}.o"
  local elf="$work_dir/${name}.elf"

  "$clang" "${compile_flags[@]}" -c "$src" -o "$obj"
  "$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
    -o "$elf" "$lib_dir/crt0.o" "$obj" "${libc_objs[@]}"
  "$lnp64_bin" elf-plan "$elf" >/dev/null

  local out
  out="$("$lnp64_bin" run-elf "$elf")"
  printf '%s\n' "$out"
  if ! grep -q "$expected_output" <<<"$out"; then
    printf 'FAIL: %s — expected output containing "%s"\n' "$name" "$expected_output" >&2
    exit 1
  fi
  printf 'PASS: %s\n' "$name"
}

# 1. fork/waitpid/wait/pthread_atfork
build_and_run fork_wait userland/fork_wait_test_clang.c "fork_wait_test ok"

# 2. SIGFPE hardware fault signal delivery
build_and_run signal_fault userland/signal_fault_test_clang.c "signal_fault_test ok"

# 3. Signal gate (software signal delivery via gate)
build_and_run signal_gate userland/signal_gate_test_clang.c "signal_gate_test ok"

# 4. POSIX timers: alarm(), timerfd, SIGALRM
build_and_run timer userland/timer_test_clang.c "timer_test ok"

# 5. mmap anonymous (brk-backed) + protection
build_and_run mmap userland/mmap_test_clang.c "mmap_test ok"

# 6. Socket loopback: bind/listen/accept/connect/poll/send/recv
build_and_run socket_loopback userland/socket_loopback_test_clang.c "socket_loopback_test ok"

printf '\nPhase C: NetBSD/POSIX personality closure VALIDATED\n'
printf 'fork/wait, signals, timers, mmap, and socket loopback all pass.\n'

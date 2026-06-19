#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_LLVM_DOCKER_IMAGE:-lnp64-real-llvm:bookworm}"
uid="$(id -u)"
gid="$(id -g)"

if [[ "${LNP64_LLVM_DOCKER_SKIP_BUILD:-0}" != "1" ]]; then
  docker build -f Dockerfile.llvm -t "$image" .
fi
docker run --rm \
  --user "$uid:$gid" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_real_llvm_lnp64.sh

if [[ "${LNP64_LLVM_DOCKER_SKIP_RUN_ELF:-0}" == "1" ]]; then
  printf 'real LLVM LNP64 run-elf execution skipped by LNP64_LLVM_DOCKER_SKIP_RUN_ELF=1\n'
  exit 0
fi

cargo build --quiet --bin lnp64
lnp64_bin="${CARGO_TARGET_DIR:-target}/debug/lnp64"
if [[ ! -x "$lnp64_bin" ]]; then
  printf 'missing built lnp64 binary: %s\n' "$lnp64_bin" >&2
  exit 1
fi

run_elf_probe() {
  local linked_probe="$1"
  shift
  "$lnp64_bin" elf-plan "$linked_probe" >/dev/null
  local run_elf_output
  run_elf_output="$("$lnp64_bin" run-elf "$linked_probe")"
  grep -q 'exit=0' <<<"$run_elf_output"
  local expected
  for expected in "$@"; do
    grep -q "$expected" <<<"$run_elf_output"
  done
}

run_elf_report() {
  local message="$1"
  local linked_probe="$2"
  shift 2
  run_elf_probe "$linked_probe" "$@"
  printf '%s: %s\n' "$message" "$linked_probe"
}

for demo in hello factorial allocator fibonacci; do
  linked_probe="target/llvm-lnp64-build/lnp64-$demo-clang-linked.elf"
  case "$demo" in
    hello) run_elf_probe "$linked_probe" 'hello from LNP64' ;;
    factorial) run_elf_probe "$linked_probe" 'factorial ok' ;;
    allocator) run_elf_probe "$linked_probe" 'alloc ok' ;;
    fibonacci) run_elf_probe "$linked_probe" 'fibonacci ok' ;;
  esac
done
printf 'real LLVM LNP64 run-elf clang demo execution passed: %s\n' \
  "target/llvm-lnp64-build/lnp64-{hello,factorial,allocator,fibonacci}-clang-linked.elf"

run_elf_report "real LLVM LNP64 run-elf native heap execution passed" \
  target/llvm-lnp64-build/lnp64-native-heap-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic push execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-push-linked.elf \
  'intrinsic push ok'
run_elf_report "real LLVM LNP64 run-elf intrinsic await execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-await-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic call execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-call-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic gate return execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-gate-return-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic control execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-control-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic capability control execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-cap-control-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic mmap execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-mmap-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic AMO execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-amo-linked.elf
run_elf_report "real LLVM LNP64 run-elf C11 atomic execution passed" \
  target/llvm-lnp64-build/lnp64-c11-atomic-linked.elf
run_elf_report "real LLVM LNP64 run-elf inline asm execution passed" \
  target/llvm-lnp64-build/lnp64-inline-asm-linked.elf
run_elf_report "real LLVM LNP64 run-elf exit execution passed" \
  target/llvm-lnp64-build/lnp64-exit-linked.elf
run_elf_report "real LLVM LNP64 run-elf errno execution passed" \
  target/llvm-lnp64-build/lnp64-errno-linked.elf
run_elf_report "real LLVM LNP64 run-elf argc execution passed" \
  target/llvm-lnp64-build/lnp64-argc-linked.elf
run_elf_report "real LLVM LNP64 run-elf startup argv/envp execution passed" \
  target/llvm-lnp64-build/lnp64-startup-linked.elf
run_elf_report "real LLVM LNP64 run-elf getauxval execution passed" \
  target/llvm-lnp64-build/lnp64-getauxval-linked.elf
run_elf_report "real LLVM LNP64 run-elf scalar arithmetic execution passed" \
  target/llvm-lnp64-build/lnp64-scalar-arith-linked.elf
run_elf_report "real LLVM LNP64 run-elf high-multiply execution passed" \
  target/llvm-lnp64-build/lnp64-high-mul-linked.elf
run_elf_report "real LLVM LNP64 run-elf scalar extension execution passed" \
  target/llvm-lnp64-build/lnp64-scalar-extend-linked.elf
run_elf_report "real LLVM LNP64 run-elf bit-manip execution passed" \
  target/llvm-lnp64-build/lnp64-bitmanip-linked.elf
run_elf_report "real LLVM LNP64 run-elf csel execution passed" \
  target/llvm-lnp64-build/lnp64-csel-linked.elf
run_elf_report "real LLVM LNP64 run-elf call-clobber execution passed" \
  target/llvm-lnp64-build/lnp64-call-clobber-linked.elf
run_elf_report "real LLVM LNP64 run-elf comparison execution passed" \
  target/llvm-lnp64-build/lnp64-compare-linked.elf
run_elf_report "real LLVM LNP64 run-elf unsigned comparison execution passed" \
  target/llvm-lnp64-build/lnp64-unsigned-compare-linked.elf
run_elf_report "real LLVM LNP64 run-elf signed-load execution passed" \
  target/llvm-lnp64-build/lnp64-signed-load-linked.elf
run_elf_report "real LLVM LNP64 run-elf wide-constant execution passed" \
  target/llvm-lnp64-build/lnp64-wide-const-linked.elf
run_elf_report "real LLVM LNP64 run-elf stack aggregate execution passed" \
  target/llvm-lnp64-build/lnp64-stack-aggregate-linked.elf
run_elf_report "real LLVM LNP64 run-elf stack-argument execution passed" \
  target/llvm-lnp64-build/lnp64-stack-args-linked.elf
run_elf_report "real LLVM LNP64 run-elf minilibc string execution passed" \
  target/llvm-lnp64-build/lnp64-libc-string-linked.elf
run_elf_report "real LLVM LNP64 run-elf numeric conversion execution passed" \
  target/llvm-lnp64-build/lnp64-convert-linked.elf
run_elf_report "real LLVM LNP64 run-elf path helper execution passed" \
  target/llvm-lnp64-build/lnp64-path-linked.elf
run_elf_report "real LLVM LNP64 run-elf search helper execution passed" \
  target/llvm-lnp64-build/lnp64-search-linked.elf
run_elf_report "real LLVM LNP64 run-elf sort helper execution passed" \
  target/llvm-lnp64-build/lnp64-sort-linked.elf
run_elf_report "real LLVM LNP64 run-elf calloc execution passed" \
  target/llvm-lnp64-build/lnp64-calloc-linked.elf
run_elf_report "real LLVM LNP64 run-elf realloc execution passed" \
  target/llvm-lnp64-build/lnp64-realloc-linked.elf
run_elf_report "real LLVM LNP64 run-elf read execution passed" \
  target/llvm-lnp64-build/lnp64-read-linked.elf
run_elf_report "real LLVM LNP64 run-elf write execution passed" \
  target/llvm-lnp64-build/lnp64-write-linked.elf \
  'fd write ok'
run_elf_report "real LLVM LNP64 run-elf mmap libc execution passed" \
  target/llvm-lnp64-build/lnp64-mmap-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf futex libc execution passed" \
  target/llvm-lnp64-build/lnp64-futex-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf poll/select/epoll/kqueue libc execution passed" \
  target/llvm-lnp64-build/lnp64-poll-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf signal libc execution passed" \
  target/llvm-lnp64-build/lnp64-signal-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf socket libc execution passed" \
  target/llvm-lnp64-build/lnp64-socket-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf indirect call execution passed" \
  target/llvm-lnp64-build/lnp64-indirect-call-linked.elf

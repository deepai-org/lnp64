#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_LLVM_DOCKER_IMAGE:-lnp64-real-llvm:bookworm}"
uid="$(id -u)"
gid="$(id -g)"

docker build -f Dockerfile.llvm -t "$image" .
docker run --rm \
  --user "$uid:$gid" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_real_llvm_lnp64.sh

for demo in hello factorial allocator fibonacci; do
  linked_probe="target/llvm-lnp64-build/lnp64-$demo-clang-linked.elf"
  cargo run --quiet -- elf-plan "$linked_probe" >/dev/null
  run_elf_output="$(cargo run --quiet -- run-elf "$linked_probe")"
  case "$demo" in
    hello) grep -q 'hello from LNP64' <<<"$run_elf_output" ;;
    factorial) grep -q 'factorial ok' <<<"$run_elf_output" ;;
    allocator) grep -q 'alloc ok' <<<"$run_elf_output" ;;
    fibonacci) grep -q 'fibonacci ok' <<<"$run_elf_output" ;;
  esac
  grep -q 'exit=0' <<<"$run_elf_output"
done
printf 'real LLVM LNP64 run-elf clang demo execution passed: %s\n' \
  "target/llvm-lnp64-build/lnp64-{hello,factorial,allocator,fibonacci}-clang-linked.elf"

heap_probe="target/llvm-lnp64-build/lnp64-native-heap-linked.elf"
cargo run --quiet -- elf-plan "$heap_probe" >/dev/null
heap_output="$(cargo run --quiet -- run-elf "$heap_probe")"
grep -q 'exit=0' <<<"$heap_output"
printf 'real LLVM LNP64 run-elf native heap execution passed: %s\n' \
  "$heap_probe"

intrinsic_probe="target/llvm-lnp64-build/lnp64-intrinsic-push-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_probe" >/dev/null
intrinsic_output="$(cargo run --quiet -- run-elf "$intrinsic_probe")"
grep -q 'intrinsic push ok' <<<"$intrinsic_output"
grep -q 'exit=0' <<<"$intrinsic_output"
printf 'real LLVM LNP64 run-elf intrinsic push execution passed: %s\n' \
  "$intrinsic_probe"

intrinsic_await_probe="target/llvm-lnp64-build/lnp64-intrinsic-await-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_await_probe" >/dev/null
intrinsic_await_output="$(cargo run --quiet -- run-elf "$intrinsic_await_probe")"
grep -q 'exit=0' <<<"$intrinsic_await_output"
printf 'real LLVM LNP64 run-elf intrinsic await execution passed: %s\n' \
  "$intrinsic_await_probe"

intrinsic_call_probe="target/llvm-lnp64-build/lnp64-intrinsic-call-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_call_probe" >/dev/null
intrinsic_call_output="$(cargo run --quiet -- run-elf "$intrinsic_call_probe")"
grep -q 'exit=0' <<<"$intrinsic_call_output"
printf 'real LLVM LNP64 run-elf intrinsic call execution passed: %s\n' \
  "$intrinsic_call_probe"

intrinsic_gate_return_probe="target/llvm-lnp64-build/lnp64-intrinsic-gate-return-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_gate_return_probe" >/dev/null
intrinsic_gate_return_output="$(cargo run --quiet -- run-elf "$intrinsic_gate_return_probe")"
grep -q 'exit=0' <<<"$intrinsic_gate_return_output"
printf 'real LLVM LNP64 run-elf intrinsic gate return execution passed: %s\n' \
  "$intrinsic_gate_return_probe"

intrinsic_ctl_probe="target/llvm-lnp64-build/lnp64-intrinsic-control-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_ctl_probe" >/dev/null
intrinsic_ctl_output="$(cargo run --quiet -- run-elf "$intrinsic_ctl_probe")"
grep -q 'exit=0' <<<"$intrinsic_ctl_output"
printf 'real LLVM LNP64 run-elf intrinsic control execution passed: %s\n' \
  "$intrinsic_ctl_probe"

intrinsic_mmap_probe="target/llvm-lnp64-build/lnp64-intrinsic-mmap-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_mmap_probe" >/dev/null
intrinsic_mmap_output="$(cargo run --quiet -- run-elf "$intrinsic_mmap_probe")"
grep -q 'exit=0' <<<"$intrinsic_mmap_output"
printf 'real LLVM LNP64 run-elf intrinsic mmap execution passed: %s\n' \
  "$intrinsic_mmap_probe"

intrinsic_amo_probe="target/llvm-lnp64-build/lnp64-intrinsic-amo-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_amo_probe" >/dev/null
intrinsic_amo_output="$(cargo run --quiet -- run-elf "$intrinsic_amo_probe")"
grep -q 'exit=0' <<<"$intrinsic_amo_output"
printf 'real LLVM LNP64 run-elf intrinsic AMO execution passed: %s\n' \
  "$intrinsic_amo_probe"

c11_atomic_probe="target/llvm-lnp64-build/lnp64-c11-atomic-linked.elf"
cargo run --quiet -- elf-plan "$c11_atomic_probe" >/dev/null
c11_atomic_output="$(cargo run --quiet -- run-elf "$c11_atomic_probe")"
grep -q 'exit=0' <<<"$c11_atomic_output"
printf 'real LLVM LNP64 run-elf C11 atomic execution passed: %s\n' \
  "$c11_atomic_probe"

inline_asm_probe="target/llvm-lnp64-build/lnp64-inline-asm-linked.elf"
cargo run --quiet -- elf-plan "$inline_asm_probe" >/dev/null
inline_asm_output="$(cargo run --quiet -- run-elf "$inline_asm_probe")"
grep -q 'exit=0' <<<"$inline_asm_output"
printf 'real LLVM LNP64 run-elf inline asm execution passed: %s\n' \
  "$inline_asm_probe"

exit_probe="target/llvm-lnp64-build/lnp64-exit-linked.elf"
cargo run --quiet -- elf-plan "$exit_probe" >/dev/null
exit_output="$(cargo run --quiet -- run-elf "$exit_probe")"
grep -q 'exit=0' <<<"$exit_output"
printf 'real LLVM LNP64 run-elf exit execution passed: %s\n' \
  "$exit_probe"

errno_probe="target/llvm-lnp64-build/lnp64-errno-linked.elf"
cargo run --quiet -- elf-plan "$errno_probe" >/dev/null
errno_output="$(cargo run --quiet -- run-elf "$errno_probe")"
grep -q 'exit=0' <<<"$errno_output"
printf 'real LLVM LNP64 run-elf errno execution passed: %s\n' \
  "$errno_probe"

argc_probe="target/llvm-lnp64-build/lnp64-argc-linked.elf"
cargo run --quiet -- elf-plan "$argc_probe" >/dev/null
argc_output="$(cargo run --quiet -- run-elf "$argc_probe")"
grep -q 'exit=0' <<<"$argc_output"
printf 'real LLVM LNP64 run-elf argc execution passed: %s\n' \
  "$argc_probe"

startup_probe="target/llvm-lnp64-build/lnp64-startup-linked.elf"
cargo run --quiet -- elf-plan "$startup_probe" >/dev/null
startup_output="$(cargo run --quiet -- run-elf "$startup_probe")"
grep -q 'exit=0' <<<"$startup_output"
printf 'real LLVM LNP64 run-elf startup argv/envp execution passed: %s\n' \
  "$startup_probe"

getauxval_probe="target/llvm-lnp64-build/lnp64-getauxval-linked.elf"
cargo run --quiet -- elf-plan "$getauxval_probe" >/dev/null
getauxval_output="$(cargo run --quiet -- run-elf "$getauxval_probe")"
grep -q 'exit=0' <<<"$getauxval_output"
printf 'real LLVM LNP64 run-elf getauxval execution passed: %s\n' \
  "$getauxval_probe"

scalar_arith_probe="target/llvm-lnp64-build/lnp64-scalar-arith-linked.elf"
cargo run --quiet -- elf-plan "$scalar_arith_probe" >/dev/null
scalar_arith_output="$(cargo run --quiet -- run-elf "$scalar_arith_probe")"
grep -q 'exit=0' <<<"$scalar_arith_output"
printf 'real LLVM LNP64 run-elf scalar arithmetic execution passed: %s\n' \
  "$scalar_arith_probe"

high_mul_probe="target/llvm-lnp64-build/lnp64-high-mul-linked.elf"
cargo run --quiet -- elf-plan "$high_mul_probe" >/dev/null
high_mul_output="$(cargo run --quiet -- run-elf "$high_mul_probe")"
grep -q 'exit=0' <<<"$high_mul_output"
printf 'real LLVM LNP64 run-elf high-multiply execution passed: %s\n' \
  "$high_mul_probe"

scalar_extend_probe="target/llvm-lnp64-build/lnp64-scalar-extend-linked.elf"
cargo run --quiet -- elf-plan "$scalar_extend_probe" >/dev/null
scalar_extend_output="$(cargo run --quiet -- run-elf "$scalar_extend_probe")"
grep -q 'exit=0' <<<"$scalar_extend_output"
printf 'real LLVM LNP64 run-elf scalar extension execution passed: %s\n' \
  "$scalar_extend_probe"

bitmanip_probe="target/llvm-lnp64-build/lnp64-bitmanip-linked.elf"
cargo run --quiet -- elf-plan "$bitmanip_probe" >/dev/null
bitmanip_output="$(cargo run --quiet -- run-elf "$bitmanip_probe")"
grep -q 'exit=0' <<<"$bitmanip_output"
printf 'real LLVM LNP64 run-elf bit-manip execution passed: %s\n' \
  "$bitmanip_probe"

csel_probe="target/llvm-lnp64-build/lnp64-csel-linked.elf"
cargo run --quiet -- elf-plan "$csel_probe" >/dev/null
csel_output="$(cargo run --quiet -- run-elf "$csel_probe")"
grep -q 'exit=0' <<<"$csel_output"
printf 'real LLVM LNP64 run-elf csel execution passed: %s\n' \
  "$csel_probe"

call_clobber_probe="target/llvm-lnp64-build/lnp64-call-clobber-linked.elf"
cargo run --quiet -- elf-plan "$call_clobber_probe" >/dev/null
call_clobber_output="$(cargo run --quiet -- run-elf "$call_clobber_probe")"
grep -q 'exit=0' <<<"$call_clobber_output"
printf 'real LLVM LNP64 run-elf call-clobber execution passed: %s\n' \
  "$call_clobber_probe"

compare_probe="target/llvm-lnp64-build/lnp64-compare-linked.elf"
cargo run --quiet -- elf-plan "$compare_probe" >/dev/null
compare_output="$(cargo run --quiet -- run-elf "$compare_probe")"
grep -q 'exit=0' <<<"$compare_output"
printf 'real LLVM LNP64 run-elf comparison execution passed: %s\n' \
  "$compare_probe"

unsigned_compare_probe="target/llvm-lnp64-build/lnp64-unsigned-compare-linked.elf"
cargo run --quiet -- elf-plan "$unsigned_compare_probe" >/dev/null
unsigned_compare_output="$(cargo run --quiet -- run-elf "$unsigned_compare_probe")"
grep -q 'exit=0' <<<"$unsigned_compare_output"
printf 'real LLVM LNP64 run-elf unsigned comparison execution passed: %s\n' \
  "$unsigned_compare_probe"

signed_load_probe="target/llvm-lnp64-build/lnp64-signed-load-linked.elf"
cargo run --quiet -- elf-plan "$signed_load_probe" >/dev/null
signed_load_output="$(cargo run --quiet -- run-elf "$signed_load_probe")"
grep -q 'exit=0' <<<"$signed_load_output"
printf 'real LLVM LNP64 run-elf signed-load execution passed: %s\n' \
  "$signed_load_probe"

wide_const_probe="target/llvm-lnp64-build/lnp64-wide-const-linked.elf"
cargo run --quiet -- elf-plan "$wide_const_probe" >/dev/null
wide_const_output="$(cargo run --quiet -- run-elf "$wide_const_probe")"
grep -q 'exit=0' <<<"$wide_const_output"
printf 'real LLVM LNP64 run-elf wide-constant execution passed: %s\n' \
  "$wide_const_probe"

stack_aggregate_probe="target/llvm-lnp64-build/lnp64-stack-aggregate-linked.elf"
cargo run --quiet -- elf-plan "$stack_aggregate_probe" >/dev/null
stack_aggregate_output="$(cargo run --quiet -- run-elf "$stack_aggregate_probe")"
grep -q 'exit=0' <<<"$stack_aggregate_output"
printf 'real LLVM LNP64 run-elf stack aggregate execution passed: %s\n' \
  "$stack_aggregate_probe"

stack_args_probe="target/llvm-lnp64-build/lnp64-stack-args-linked.elf"
cargo run --quiet -- elf-plan "$stack_args_probe" >/dev/null
stack_args_output="$(cargo run --quiet -- run-elf "$stack_args_probe")"
grep -q 'exit=0' <<<"$stack_args_output"
printf 'real LLVM LNP64 run-elf stack-argument execution passed: %s\n' \
  "$stack_args_probe"

libc_string_probe="target/llvm-lnp64-build/lnp64-libc-string-linked.elf"
cargo run --quiet -- elf-plan "$libc_string_probe" >/dev/null
libc_string_output="$(cargo run --quiet -- run-elf "$libc_string_probe")"
grep -q 'exit=0' <<<"$libc_string_output"
printf 'real LLVM LNP64 run-elf minilibc string execution passed: %s\n' \
  "$libc_string_probe"

calloc_probe="target/llvm-lnp64-build/lnp64-calloc-linked.elf"
cargo run --quiet -- elf-plan "$calloc_probe" >/dev/null
calloc_output="$(cargo run --quiet -- run-elf "$calloc_probe")"
grep -q 'exit=0' <<<"$calloc_output"
printf 'real LLVM LNP64 run-elf calloc execution passed: %s\n' \
  "$calloc_probe"

realloc_probe="target/llvm-lnp64-build/lnp64-realloc-linked.elf"
cargo run --quiet -- elf-plan "$realloc_probe" >/dev/null
realloc_output="$(cargo run --quiet -- run-elf "$realloc_probe")"
grep -q 'exit=0' <<<"$realloc_output"
printf 'real LLVM LNP64 run-elf realloc execution passed: %s\n' \
  "$realloc_probe"

read_probe="target/llvm-lnp64-build/lnp64-read-linked.elf"
cargo run --quiet -- elf-plan "$read_probe" >/dev/null
read_output="$(cargo run --quiet -- run-elf "$read_probe")"
grep -q 'exit=0' <<<"$read_output"
printf 'real LLVM LNP64 run-elf read execution passed: %s\n' \
  "$read_probe"

mmap_libc_probe="target/llvm-lnp64-build/lnp64-mmap-libc-linked.elf"
cargo run --quiet -- elf-plan "$mmap_libc_probe" >/dev/null
mmap_libc_output="$(cargo run --quiet -- run-elf "$mmap_libc_probe")"
grep -q 'exit=0' <<<"$mmap_libc_output"
printf 'real LLVM LNP64 run-elf mmap libc execution passed: %s\n' \
  "$mmap_libc_probe"

indirect_call_probe="target/llvm-lnp64-build/lnp64-indirect-call-linked.elf"
cargo run --quiet -- elf-plan "$indirect_call_probe" >/dev/null
indirect_call_output="$(cargo run --quiet -- run-elf "$indirect_call_probe")"
grep -q 'exit=0' <<<"$indirect_call_output"
printf 'real LLVM LNP64 run-elf indirect call execution passed: %s\n' \
  "$indirect_call_probe"

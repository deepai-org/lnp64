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

intrinsic_probe="target/llvm-lnp64-build/lnp64-intrinsic-push-linked.elf"
cargo run --quiet -- elf-plan "$intrinsic_probe" >/dev/null
intrinsic_output="$(cargo run --quiet -- run-elf "$intrinsic_probe")"
grep -q 'intrinsic push ok' <<<"$intrinsic_output"
grep -q 'exit=0' <<<"$intrinsic_output"
printf 'real LLVM LNP64 run-elf intrinsic push execution passed: %s\n' \
  "$intrinsic_probe"

exit_probe="target/llvm-lnp64-build/lnp64-exit-linked.elf"
cargo run --quiet -- elf-plan "$exit_probe" >/dev/null
exit_output="$(cargo run --quiet -- run-elf "$exit_probe")"
grep -q 'exit=0' <<<"$exit_output"
printf 'real LLVM LNP64 run-elf exit execution passed: %s\n' \
  "$exit_probe"

argc_probe="target/llvm-lnp64-build/lnp64-argc-linked.elf"
cargo run --quiet -- elf-plan "$argc_probe" >/dev/null
argc_output="$(cargo run --quiet -- run-elf "$argc_probe")"
grep -q 'exit=0' <<<"$argc_output"
printf 'real LLVM LNP64 run-elf argc execution passed: %s\n' \
  "$argc_probe"

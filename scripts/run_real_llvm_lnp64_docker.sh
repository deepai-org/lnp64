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

linked_probe="target/llvm-lnp64-build/lnp64-hello-clang-linked.elf"
cargo run --quiet -- elf-plan "$linked_probe" >/dev/null
set +e
run_elf_output="$(cargo run --quiet -- run-elf "$linked_probe" 2>&1)"
run_elf_status=$?
set -e
if [[ "$run_elf_status" -eq 0 ]]; then
  printf 'run-elf unexpectedly executed linked Clang ELF without decode gate\n' >&2
  exit 1
fi
grep -q 'ELF text fetch/decode is not implemented yet' <<<"$run_elf_output"
printf 'real LLVM LNP64 run-elf linked hello decode gate passed: %s\n' \
  "$linked_probe"

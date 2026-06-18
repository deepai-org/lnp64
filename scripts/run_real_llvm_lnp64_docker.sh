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
cargo run --quiet -- run-elf "$linked_probe" >/dev/null
printf 'real LLVM LNP64 run-elf linked hello execution passed: %s\n' \
  "$linked_probe"

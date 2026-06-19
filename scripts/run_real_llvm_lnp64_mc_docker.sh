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
  -e LNP64_LLVM_GATE=mc \
  -e LNP64_LLVM_JOBS="${LNP64_LLVM_JOBS:-}" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_real_llvm_lnp64.sh

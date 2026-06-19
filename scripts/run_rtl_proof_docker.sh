#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_RTL_PROOF_IMAGE:-lnp64-rtl-proof}"
lean_toolchain="${LNP64_LEAN_TOOLCHAIN:-stable}"
build_gates="${LNP64_RTL_PROOF_BUILD_GATES:-0}"
skip_build="${LNP64_RTL_PROOF_SKIP_BUILD:-0}"

docker_env=(
  -e LNP64_REQUIRE_LEAN=1
)

for var in \
  LNP64_RTL_PROOF_RANDOM_COSIM \
  LNP64_RTL_PROOF_SKIP_RANDOM_COSIM \
  LNP64_RTL_RANDOM_COSIM_JOBS \
  LNP64_COSIM_SEEDS
do
  if [[ -n "${!var+x}" ]]; then
    docker_env+=(-e "${var}=${!var}")
  fi
done

if [[ "$skip_build" == "1" ]]; then
  printf 'using existing RTL/proof Docker image %s (LNP64_RTL_PROOF_SKIP_BUILD=1)\n' "$image"
else
  docker build \
    -f Dockerfile.rtl-proof \
    --build-arg "LEAN_TOOLCHAIN=${lean_toolchain}" \
    --build-arg "RUN_RTL_PROOF_GATES=${build_gates}" \
    -t "$image" \
    .
fi

docker run --rm \
  "${docker_env[@]}" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_rtl_proof_gates.sh

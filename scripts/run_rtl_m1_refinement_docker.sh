#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_RTL_PROOF_IMAGE:-lnp64-rtl-proof}"
lean_toolchain="${LNP64_LEAN_TOOLCHAIN:-stable}"
build_gates="${LNP64_RTL_PROOF_BUILD_GATES:-0}"
skip_build="${LNP64_RTL_PROOF_SKIP_BUILD:-0}"
rebuild_image="${LNP64_RTL_PROOF_REBUILD_IMAGE:-0}"

docker_env=(
  -e LNP64_REQUIRE_LEAN=1
)

for var in \
  LNP64_RTL_FAST \
  LNP64_RTL_REUSE_BUILD \
  LNP64_RTL_SKIP_LINT \
  LNP64_RTL_SKIP_BUILD \
  LNP64_RTL_BUILD_ROOT \
  LNP64_RTL_COSIM_SEED_JOBS \
  LNP64_RTL_COSIM_KEEP_SEED_LOGS \
  LNP64_COSIM_SEEDS \
  LNP64_M1_TYPED_COMMIT_SEEDS
do
  if [[ -n "${!var+x}" ]]; then
    docker_env+=(-e "${var}=${!var}")
  fi
done

image_exists=0
if docker image inspect "$image" >/dev/null 2>&1; then
  image_exists=1
fi

if [[ "$skip_build" == "1" ]] ||
   [[ "$rebuild_image" != "1" && "$image_exists" == "1" ]]; then
  printf 'using existing RTL/proof Docker image %s\n' "$image"
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
  bash scripts/run_rtl_m1_refinement_gate.sh

if [[ "${LNP64_M1_REFINEMENT_SKIP_TOP:-0}" != "1" ]]; then
  LNP64_RTL_FAST="${LNP64_RTL_FAST:-1}" \
  LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}" \
  LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}" \
  LNP64_RTL_TOP_PROGRAM_FILTER="${LNP64_M1_REFINEMENT_TOP_FILTER:-*top_cap* tests/rtl/programs/top_pipe_push_pull.s tests/rtl/programs/top_pipe_static_push_pull.s demos/capability_transfer.s}" \
  LNP64_RTL_TOP_PROGRAM_JOBS="${LNP64_RTL_TOP_PROGRAM_JOBS:-auto}" \
  LNP64_RTL_TOP_PROGRAM_QUIET="${LNP64_RTL_TOP_PROGRAM_QUIET:-1}" \
    bash scripts/run_rtl_execution_fast_docker.sh
fi

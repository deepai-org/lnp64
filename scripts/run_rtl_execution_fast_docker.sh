#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_RTL_EXEC_IMAGE:-lnp64-rtl-exec}"
build_gates="${LNP64_RTL_EXEC_BUILD_GATES:-0}"
skip_build="${LNP64_RTL_EXEC_SKIP_BUILD:-0}"
rebuild_image="${LNP64_RTL_EXEC_REBUILD_IMAGE:-0}"

docker_env=()
docker_env+=(-e "CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/work/target/docker-rust}")
docker_env+=(-e "LNP64_RTL_BUILD_ROOT=${LNP64_RTL_BUILD_ROOT:-/work/target/rtl-verilator-docker}")
for var in \
  CARGO_TARGET_DIR \
  LNP64_RTL_FAST \
  LNP64_RTL_REUSE_BUILD \
  LNP64_RTL_SKIP_LINT \
  LNP64_RTL_SKIP_BUILD \
  LNP64_RTL_VERILATOR_BUILD_JOBS \
  LNP64_RTL_TOP_PROGRAM_JOBS \
  LNP64_RTL_TOP_PROGRAM_FILTER \
  LNP64_RTL_TOP_PROGRAM_KEEP_LOGS \
  LNP64_RTL_TOP_PROGRAM_MAX_CYCLES \
  LNP64_RTL_TOP_PROGRAM_QUIET \
  LNP64_RTL_TOP_PROGRAM_SKIP_BUILD
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
  printf 'using existing RTL execution Docker image %s\n' "$image"
else
  docker build \
    -f Dockerfile.rtl-exec \
    --build-arg "RUN_RTL_EXEC_GATES=${build_gates}" \
    -t "$image" \
    .
fi

docker run --rm \
  "${docker_env[@]}" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_rtl_execution_fast.sh "$@"

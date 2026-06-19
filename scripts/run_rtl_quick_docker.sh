#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

usage() {
  cat <<'USAGE'
usage: scripts/run_rtl_quick_docker.sh [top|s0|m1|cosim|proof] [args...]

Fast Docker-backed RTL inner loop.

Modes:
  top    Run top-level program manifest, or the explicit program args.
  s0     Run the S0 RTL smoke.
  m1     Run the M1 RTL/cosim gate.
  cosim  Run narrowed randomized/cosim; extra args become gate names.
  proof  Run fast RTL/proof gates in the Lean-capable Docker image.

Set LNP64_RTL_QUICK_REBUILD_IMAGE=1 to rebuild the Docker image.
Set LNP64_RTL_RANDOM_COSIM_GATES/LNP64_COSIM_SEEDS to widen cosim evidence.
USAGE
}

mode="top"
if [[ "$#" -gt 0 ]]; then
  case "$1" in
    top|s0|m1|cosim|proof)
      mode="$1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
  esac
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/work/target/docker-rust}"
export LNP64_RTL_BUILD_ROOT="${LNP64_RTL_BUILD_ROOT:-/work/target/rtl-verilator-docker}"
export LNP64_RTL_FAST="${LNP64_RTL_FAST:-1}"
export LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}"
export LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}"
export LNP64_RTL_VERILATOR_BUILD_JOBS="${LNP64_RTL_VERILATOR_BUILD_JOBS:-auto}"
export LNP64_RTL_TOP_PROGRAM_JOBS="${LNP64_RTL_TOP_PROGRAM_JOBS:-auto}"
export LNP64_RTL_TOP_PROGRAM_QUIET="${LNP64_RTL_TOP_PROGRAM_QUIET:-1}"
export LNP64_COSIM_SEEDS="${LNP64_COSIM_SEEDS:-0}"
export LNP64_RTL_COSIM_SEED_JOBS="${LNP64_RTL_COSIM_SEED_JOBS:-auto}"

image="lnp64-rtl-exec"
dockerfile="Dockerfile.rtl-exec"
command=(bash scripts/run_rtl_execution_fast.sh "$@")

case "$mode" in
  top)
    command=(bash scripts/run_rtl_execution_fast.sh "$@")
    ;;
  s0)
    command=(bash scripts/run_rtl_s0.sh "$@")
    ;;
  m1)
    command=(bash scripts/run_rtl_m1.sh "$@")
    ;;
  cosim)
    export LNP64_RTL_RANDOM_COSIM_JOBS="${LNP64_RTL_RANDOM_COSIM_JOBS:-auto}"
    if [[ "$#" -gt 0 ]]; then
      export LNP64_RTL_RANDOM_COSIM_GATES="$*"
    else
      export LNP64_RTL_RANDOM_COSIM_GATES="${LNP64_RTL_RANDOM_COSIM_GATES:-m1}"
    fi
    command=(bash scripts/run_rtl_random_cosim.sh)
    ;;
  proof)
    image="${LNP64_RTL_PROOF_IMAGE:-lnp64-rtl-proof}"
    dockerfile="Dockerfile.rtl-proof"
    export LNP64_REQUIRE_LEAN=1
    export LNP64_RTL_PROOF_RANDOM_COSIM="${LNP64_RTL_PROOF_RANDOM_COSIM:-0}"
    command=(bash scripts/run_rtl_proof_gates.sh "$@")
    ;;
esac

if [[ "${LNP64_RTL_QUICK_REBUILD_IMAGE:-0}" == "1" ]] ||
   ! docker image inspect "$image" >/dev/null 2>&1; then
  docker build -f "$dockerfile" -t "$image" .
else
  printf 'using existing Docker image %s\n' "$image"
fi

docker run --rm \
  -e "CARGO_TARGET_DIR=$CARGO_TARGET_DIR" \
  -e "LNP64_RTL_BUILD_ROOT=$LNP64_RTL_BUILD_ROOT" \
  -e "LNP64_RTL_FAST=$LNP64_RTL_FAST" \
  -e "LNP64_RTL_REUSE_BUILD=$LNP64_RTL_REUSE_BUILD" \
  -e "LNP64_RTL_SKIP_LINT=$LNP64_RTL_SKIP_LINT" \
  -e "LNP64_RTL_SKIP_BUILD=${LNP64_RTL_SKIP_BUILD:-}" \
  -e "LNP64_RTL_VERILATOR_BUILD_JOBS=$LNP64_RTL_VERILATOR_BUILD_JOBS" \
  -e "LNP64_RTL_TOP_PROGRAM_JOBS=$LNP64_RTL_TOP_PROGRAM_JOBS" \
  -e "LNP64_RTL_TOP_PROGRAM_FILTER=${LNP64_RTL_TOP_PROGRAM_FILTER:-}" \
  -e "LNP64_RTL_TOP_PROGRAM_KEEP_LOGS=${LNP64_RTL_TOP_PROGRAM_KEEP_LOGS:-}" \
  -e "LNP64_RTL_TOP_PROGRAM_MAX_CYCLES=${LNP64_RTL_TOP_PROGRAM_MAX_CYCLES:-}" \
  -e "LNP64_RTL_TOP_PROGRAM_QUIET=$LNP64_RTL_TOP_PROGRAM_QUIET" \
  -e "LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=${LNP64_RTL_TOP_PROGRAM_SKIP_BUILD:-}" \
  -e "LNP64_COSIM_SEEDS=$LNP64_COSIM_SEEDS" \
  -e "LNP64_RTL_COSIM_SEED_JOBS=$LNP64_RTL_COSIM_SEED_JOBS" \
  -e "LNP64_RTL_RANDOM_COSIM_JOBS=${LNP64_RTL_RANDOM_COSIM_JOBS:-}" \
  -e "LNP64_RTL_RANDOM_COSIM_GATES=${LNP64_RTL_RANDOM_COSIM_GATES:-}" \
  -e "LNP64_RTL_PROOF_RANDOM_COSIM=${LNP64_RTL_PROOF_RANDOM_COSIM:-}" \
  -e "LNP64_RTL_PROOF_GATE_JOBS=${LNP64_RTL_PROOF_GATE_JOBS:-}" \
  -e "LNP64_RTL_PROOF_KEEP_GATE_LOGS=${LNP64_RTL_PROOF_KEEP_GATE_LOGS:-}" \
  -e "LNP64_REQUIRE_LEAN=${LNP64_REQUIRE_LEAN:-}" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  "${command[@]}"

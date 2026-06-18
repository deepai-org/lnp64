#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

require_board=0
docker_mode=none
board_evidence="${LNP64_BOARD_EVIDENCE:-build/lnp64-board-ice40-s0-evidence.json}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --require-board-evidence)
      require_board=1
      ;;
    --docker-rerun)
      docker_mode=rerun
      ;;
    --docker-build)
      docker_mode=build
      ;;
    --board-evidence)
      shift
      if [ "$#" -eq 0 ]; then
        printf '%s\n' "--board-evidence requires a path" >&2
        exit 2
      fi
      board_evidence="$1"
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

case "$docker_mode" in
  none)
    ;;
  rerun)
    docker run --rm \
      -e LNP64_REQUIRE_LEAN=1 \
      -v "$root:/work" \
      -w /work \
      "${LNP64_RTL_PROOF_IMAGE:-lnp64-rtl-proof}" \
      bash scripts/run_rtl_proof_gates.sh
    docker run --rm \
      -v "$root:/work" \
      -w /work \
      "${LNP64_RTL_SYNTH_IMAGE:-lnp64-rtl-synth}" \
      bash scripts/run_rtl_synth_gates.sh
    ;;
  build)
    bash scripts/run_rtl_proof_docker.sh
    bash scripts/run_rtl_synth_docker.sh
    ;;
  *)
    printf 'invalid docker mode: %s\n' "$docker_mode" >&2
    exit 2
    ;;
esac

scripts/check_formal_proof_manifest.py
scripts/check_rtl_cosim_manifest.py
scripts/check_rtl_synth_constraints.py
scripts/check_fpga_bringup_manifest.py
scripts/check_rtl_track_b_manifest.py
scripts/check_rtl_s0_contract.py
scripts/check_rtl_dockerfiles.py

if [ "$require_board" -eq 1 ]; then
  LNP64_REQUIRE_BOARD_EVIDENCE=1 \
    scripts/check_formal_rtl_roadmap_audit.py \
    --board-evidence "$board_evidence"
else
  scripts/check_formal_rtl_roadmap_audit.py \
    --board-evidence "$board_evidence"
fi

printf '%s\n' "formal RTL roadmap audit gate ok"

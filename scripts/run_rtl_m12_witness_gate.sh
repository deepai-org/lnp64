#!/usr/bin/env bash
# Produce and consume the M12 RAS refinement witness artifact.
#
# Runs the seed-0 M12 typed VMA gate with witness emission enabled, then
# re-validates the generated lnp64_m12_ddr_refinement_witness_v1 artifact offline
# with scripts/check_rtl_m12_witness.py. Mirrors the M1/M7 witness gates.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

witness_out="${LNP64_RTL_M12_WITNESS_OUT:-$root/build/lnp64-m12-storage-refinement-witness.json}"

# Offline consumer self-test first: it is hermetic and fails fast.
scripts/test_rtl_m12_witness_checker.py

LNP64_RTL_M12_WITNESS_OUT="$witness_out" \
  scripts/check_rtl_m12_typed_commit_trace.py

scripts/check_rtl_m12_witness.py "$witness_out"

printf '%s\n' "rtl m12 witness gate ok"

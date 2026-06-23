#!/usr/bin/env bash
# Produce and consume the M16 endpoint refinement witness artifact.
#
# Runs the seed-0 M16 typed endpoint gate with witness emission enabled, then
# re-validates the generated lnp64_m16_endpoint_refinement_witness_v1 artifact
# offline with scripts/check_rtl_m16_witness.py. Mirrors the M1/M7/M15 witness
# gates.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

witness_out="${LNP64_RTL_M16_WITNESS_OUT:-$root/build/lnp64-m16-endpoint-refinement-witness.json}"

# Offline consumer self-test first: it is hermetic and fails fast.
scripts/test_rtl_m16_witness_checker.py

LNP64_RTL_M16_WITNESS_OUT="$witness_out" \
  scripts/check_rtl_m16_typed_commit_trace.py

scripts/check_rtl_m16_witness.py "$witness_out"

printf '%s\n' "rtl m16 witness gate ok"

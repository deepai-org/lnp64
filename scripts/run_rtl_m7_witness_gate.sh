#!/usr/bin/env bash
# Produce and consume the M7 scheduler/wakeup refinement witness artifact.
#
# Runs the seed-0 M7 typed scheduler gate with witness emission enabled, then
# re-validates the generated lnp64_m7_sched_refinement_witness_v1 artifact
# offline with scripts/check_rtl_m7_witness.py. Mirrors the M1 top-level
# witness gate so the scheduler refinement evidence is a consumed, auditable
# artifact rather than orphaned plumbing.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

witness_out="${LNP64_RTL_M7_WITNESS_OUT:-$root/build/lnp64-m7-sched-refinement-witness.json}"

# Offline consumer self-test first: it is hermetic and fails fast.
scripts/test_rtl_m7_witness_checker.py

LNP64_RTL_M7_WITNESS_OUT="$witness_out" \
  scripts/check_rtl_m7_typed_commit_trace.py

scripts/check_rtl_m7_witness.py "$witness_out"

printf '%s\n' "rtl m7 witness gate ok"

#!/usr/bin/env bash
# Produce and consume the lnp64_top M1 refinement witness artifact.
#
# Runs a single deterministic top-level program through lnp64_top with witness
# emission enabled, then re-validates the generated
# lnp64_top_m1_refinement_witness_v1 artifact offline with
# scripts/check_rtl_top_m1_witness.py. This keeps the generated witness a
# consumed, auditable artifact rather than orphaned evidence plumbing.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

program="${LNP64_RTL_TOP_M1_WITNESS_PROGRAM:-demos/object_profiles.s}"
witness_out="${LNP64_RTL_TOP_M1_WITNESS_OUT:-$root/build/lnp64-top-m1-refinement-witness.json}"

# Offline checker self-test first: it is hermetic and fails fast.
scripts/test_rtl_top_m1_witness_checker.py

export LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}"
export LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}"
export LNP64_RTL_BUILD_ROOT="${LNP64_RTL_BUILD_ROOT:-$root/target/rtl-verilator}"
export LNP64_RTL_TOP_M1_WITNESS_OUT="$witness_out"

bash scripts/run_rtl_top_program_smoke.sh "$program"

scripts/check_rtl_top_m1_witness.py "$witness_out"

printf '%s\n' "rtl top-level M1 witness gate ok"

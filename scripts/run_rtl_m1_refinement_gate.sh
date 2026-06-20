#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ "${LNP64_RTL_FAST:-0}" == "1" ]]; then
  export LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}"
  export LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}"
  export LNP64_RTL_BUILD_ROOT="${LNP64_RTL_BUILD_ROOT:-$root/target/rtl-verilator}"
fi

lean_file="formal/M1TransitionInvariantModel.lean"

if grep -RInE '(^|[^[:alnum:]_])(axiom|sorry|admit)([^[:alnum:]_]|$)' "$lean_file"; then
  printf '%s\n' "M1 Lean refinement model must not contain axiom, sorry, or admit" >&2
  exit 1
fi

if command -v lean >/dev/null 2>&1 && lean --version >/dev/null 2>&1; then
  lean "$lean_file"
elif [[ "${LNP64_REQUIRE_LEAN:-0}" == "1" ]]; then
  printf '%s\n' "lean is required for this gate but is not configured" >&2
  exit 1
else
  printf '%s\n' "lean not configured; skipping M1 Lean syntax check (set LNP64_REQUIRE_LEAN=1 to require it)"
fi

scripts/check_rtl_shared_schema.py
scripts/check_theorem_rtl_coupling.py
formal/m1_model.py >/dev/null

m1_log="${TMPDIR:-/tmp}/lnp64_rtl_m1_refinement_gate.log"
if [[ "${LNP64_RTL_FAST:-0}" == "1" ]]; then
  default_m1_seeds="0"
else
  default_m1_seeds="0 1 7 42 255 1024 4095 4096 65536 1048576 16777216 134217728 268435456 536870912"
fi
export LNP64_COSIM_SEEDS="${LNP64_COSIM_SEEDS:-${LNP64_M1_TYPED_COMMIT_SEEDS:-$default_m1_seeds}}"
bash scripts/run_rtl_m1.sh | tee "$m1_log"
LNP64_M1_TYPED_COMMIT_USE_EXISTING=1 \
  LNP64_M1_TYPED_COMMIT_LOG="$m1_log" \
  scripts/check_rtl_m1_typed_commit_trace.py
scripts/test_rtl_m1_typed_commit_checker.py
scripts/test_rtl_m1_schema_checker.py
scripts/test_rtl_top_m1_witness_checker.py

printf '%s\n' "rtl m1 refinement gate ok"

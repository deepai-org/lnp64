#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

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

bash scripts/run_rtl_m1.sh
scripts/check_rtl_m1_typed_commit_trace.py
scripts/test_rtl_m1_typed_commit_checker.py
scripts/test_rtl_m1_schema_checker.py

printf '%s\n' "rtl m1 refinement gate ok"

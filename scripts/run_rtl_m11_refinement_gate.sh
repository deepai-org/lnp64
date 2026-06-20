#!/usr/bin/env bash
# Check the M11 RTL-to-Lean refinement slice.
#
# Concatenates formal/M11TransitionInvariantModel.lean ahead of
# formal/M11RtlRefinement.lean (same Lnp64.M11Transition namespace resolves) and
# checks with `lean`. Proves each well-formed emitted typed-commit op is exactly
# one Lnp64.M11Transition.Step, the emitted seed-0 op trace is a Reachable path,
# and the resulting state satisfies the proved transition invariant. Kernel
# tactics only (no native_decide/sorry/admit).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v lean >/dev/null 2>&1 || ! lean --version >/dev/null 2>&1; then
  if [[ "${LNP64_REQUIRE_LEAN:-0}" == "1" ]]; then
    printf '%s\n' "lean is required for the M11 refinement gate but is not configured" >&2
    exit 1
  fi
  printf '%s\n' "lean not configured; skipping M11 refinement gate (set LNP64_REQUIRE_LEAN=1 to require it)"
  exit 0
fi

if grep -nE '(^|[^[:alnum:]_])(native_decide|sorry|admit)([^[:alnum:]_]|$)' formal/M11RtlRefinement.lean; then
  printf '%s\n' "M11 refinement must use kernel tactics only (no native_decide/sorry/admit)" >&2
  exit 1
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/lnp64-m11-refinement.XXXXXX")"
trap 'rm -rf "$work"' EXIT
combined="$work/m11_refinement_combined.lean"

cat formal/M11TransitionInvariantModel.lean > "$combined"
printf '\n' >> "$combined"
cat formal/M11RtlRefinement.lean >> "$combined"

lean "$combined"

printf '%s\n' "rtl m11 refinement gate ok"

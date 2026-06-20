#!/usr/bin/env bash
# Check the LNP64 whole-chip composition proof.
#
# Concatenates the fifteen per-engine transition-invariant models
# (formal/M*TransitionInvariantModel.lean) ahead of
# formal/WholeChipComposition.lean -- each lives in its own Lnp64.M*Transition
# namespace, so a single combined compilation unit resolves every reference --
# and checks the result with `lean`. Proves that the conjunction of all fifteen
# engines' severe-goal transition invariants holds in every reachable whole-chip
# state, with kernel tactics only (no native_decide/sorry/admit).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v lean >/dev/null 2>&1 || ! lean --version >/dev/null 2>&1; then
  if [[ "${LNP64_REQUIRE_LEAN:-0}" == "1" ]]; then
    printf '%s\n' "lean is required for the whole-chip composition gate but is not configured" >&2
    exit 1
  fi
  printf '%s\n' "lean not configured; skipping whole-chip composition gate (set LNP64_REQUIRE_LEAN=1 to require it)"
  exit 0
fi

engines=(M1 M2 M3 M4 M5 M6 M7 M8 M9 M10 M11 M12 M13 M14 M15)

work="$(mktemp -d "${TMPDIR:-/tmp}/lnp64-whole-chip.XXXXXX")"
trap 'rm -rf "$work"' EXIT
combined="$work/whole_chip_combined.lean"

: > "$combined"
for m in "${engines[@]}"; do
  cat "formal/${m}TransitionInvariantModel.lean" >> "$combined"
  printf '\n' >> "$combined"
done
cat formal/WholeChipComposition.lean >> "$combined"

if grep -nE '(^|[^[:alnum:]_])(native_decide|sorry|admit)([^[:alnum:]_]|$)' formal/WholeChipComposition.lean; then
  printf '%s\n' "whole-chip composition must use kernel tactics only (no native_decide/sorry/admit)" >&2
  exit 1
fi

lean "$combined"

printf '%s\n' "rtl whole chip composition gate ok"

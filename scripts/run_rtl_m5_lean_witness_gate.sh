#!/usr/bin/env bash
# Prove decode-faithfulness of the emitted M5 scheduler witness in Lean.
#
# Generates Lean `decide` examples from the lnp64_m5_sched_refinement_witness_v1
# artifact (scripts/gen_m5_witness_lean.py), concatenates them after the
# standalone formal/M5TransitionInvariantModel.lean model (no lakefile), and
# checks the result with `lean`. Proves every emitted commit/state packed field
# decodes under the Lnp64.M5Transition packed-bit decoders to the recorded
# value, with the kernel `decide` tactic (no native_decide, no axioms).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

model="formal/M5TransitionInvariantModel.lean"
witness_out="${LNP64_RTL_M5_WITNESS_OUT:-$root/build/lnp64-m5-dma-refinement-witness.json}"

if ! command -v lean >/dev/null 2>&1 || ! lean --version >/dev/null 2>&1; then
  if [[ "${LNP64_REQUIRE_LEAN:-0}" == "1" ]]; then
    printf '%s\n' "lean is required for the M5 Lean witness gate but is not configured" >&2
    exit 1
  fi
  printf '%s\n' "lean not configured; skipping M5 Lean witness decode-faithfulness gate (set LNP64_REQUIRE_LEAN=1 to require it)"
  exit 0
fi

if [[ ! -f "$witness_out" ]]; then
  if command -v verilator >/dev/null 2>&1; then
    LNP64_RTL_M5_WITNESS_OUT="$witness_out" \
      bash scripts/run_rtl_m5_witness_gate.sh >/dev/null
  else
    printf '%s\n' "missing M5 witness $witness_out and no verilator to produce it" >&2
    exit 1
  fi
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/lnp64-m5-lean-witness.XXXXXX")"
trap 'rm -rf "$work"' EXIT
generated="$work/m5_witness_gen.lean"
combined="$work/m5_combined.lean"

scripts/gen_m5_witness_lean.py "$witness_out" > "$generated"

if grep -nE '(^|[^[:alnum:]_])(native_decide|axiom|sorry|admit)([^[:alnum:]_]|$)' "$generated"; then
  printf '%s\n' "generated M5 witness Lean must use kernel decide only (no native_decide/axiom/sorry/admit)" >&2
  exit 1
fi

cat "$model" "$generated" > "$combined"
lean "$combined"

printf '%s\n' "rtl m5 lean witness gate ok"

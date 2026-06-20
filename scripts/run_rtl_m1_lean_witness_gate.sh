#!/usr/bin/env bash
# Prove decode-faithfulness of the emitted lnp64_top M1 witness in Lean.
#
# Generates Lean `decide` examples from the lnp64_top_m1_refinement_witness_v1
# artifact (scripts/gen_m1_witness_lean.py), concatenates them after the
# standalone formal/M1TransitionInvariantModel.lean model (no lakefile), and
# checks the result with `lean`. This is the first artifact connecting the
# *emitted* RTL packed bit vectors to the *Lean* packed-bit decoders; it proves
# every emitted commit/pre/post field, op tag, and status tag decodes to the
# recorded projection value with the kernel `decide` tactic (no native_decide,
# no axioms).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

model="formal/M1TransitionInvariantModel.lean"
witness_out="${LNP64_RTL_TOP_M1_WITNESS_OUT:-$root/build/lnp64-top-m1-refinement-witness.json}"

if ! command -v lean >/dev/null 2>&1 || ! lean --version >/dev/null 2>&1; then
  if [[ "${LNP64_REQUIRE_LEAN:-0}" == "1" ]]; then
    printf '%s\n' "lean is required for the M1 Lean witness gate but is not configured" >&2
    exit 1
  fi
  printf '%s\n' "lean not configured; skipping M1 Lean witness decode-faithfulness gate (set LNP64_REQUIRE_LEAN=1 to require it)"
  exit 0
fi

if [[ ! -f "$witness_out" ]]; then
  # Inside the proof image there is no Verilator; the witness is produced by the
  # exec-image witness step earlier in run_rtl_m1_refinement_docker.sh. Only
  # self-produce when a simulator is actually available.
  if command -v verilator >/dev/null 2>&1; then
    LNP64_RTL_TOP_M1_WITNESS_OUT="$witness_out" \
    LNP64_RTL_FAST="${LNP64_RTL_FAST:-1}" \
    LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}" \
    LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}" \
      bash scripts/run_rtl_top_m1_witness_gate.sh >/dev/null
  else
    printf '%s\n' "missing M1 witness $witness_out and no verilator to produce it" >&2
    exit 1
  fi
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/lnp64-m1-lean-witness.XXXXXX")"
trap 'rm -rf "$work"' EXIT
generated="$work/m1_witness_gen.lean"
combined="$work/m1_combined.lean"

scripts/gen_m1_witness_lean.py "$witness_out" > "$generated"

if grep -nE '(^|[^[:alnum:]_])(native_decide|axiom|sorry|admit)([^[:alnum:]_]|$)' "$generated"; then
  printf '%s\n' "generated M1 witness Lean must use kernel decide only (no native_decide/axiom/sorry/admit)" >&2
  exit 1
fi

cat "$model" "$generated" > "$combined"
lean "$combined"

printf '%s\n' "rtl m1 lean witness gate ok"

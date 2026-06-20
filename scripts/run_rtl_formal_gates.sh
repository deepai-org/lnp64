#!/usr/bin/env bash
# Run all LNP64 RTL formal property-verification gates.
#
# Each gate model-checks an engine's severe-goal SVA assertions on the actual
# synthesized SystemVerilog with yosys + yosys-smtbmc (BMC + temporal
# k-induction), proving the severe goals over ALL input sequences rather than a
# single simulation trace. Use the lnp64-rtl-formal image.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

gates=(
  scripts/run_rtl_m13_formal.sh
)

for gate in "${gates[@]}"; do
  printf '\n=== %s ===\n' "$gate"
  bash "$gate"
done

printf '%s\n' "rtl formal gates ok"

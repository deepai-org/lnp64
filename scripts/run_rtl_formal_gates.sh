#!/usr/bin/env bash
# Run all LNP64 RTL formal property-verification gates.
#
# Each engine listed here has a formal wrapper rtl/formal/lnp64_<engine>_formal.sv
# whose severe-goal SVA assertions are model-checked on the actual synthesized
# SystemVerilog with yosys + yosys-smtbmc (BMC + temporal k-induction), proving
# the severe goals over ALL input sequences rather than a single simulation
# trace. Use the lnp64-rtl-formal image.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

engines=(
  m12
  m13
)

for engine in "${engines[@]}"; do
  printf '\n========== formal: %s ==========\n' "$engine"
  bash scripts/run_rtl_formal.sh "$engine"
done

printf '%s\n' "rtl formal gates ok"

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
  m8
  m9
  m10
  m11
  m12
  m13
  m15
)

# Small engine-shell "does what it says" proofs: <name> <shell module...>
shell_proofs=(
  "fail_closed lnp64_fail_closed_engine"
  "watchdog lnp64_watchdog"
  "policy_engine lnp64_policy_engine"
  "completion_router lnp64_completion_router"
)

for engine in "${engines[@]}"; do
  printf '\n========== formal: %s ==========\n' "$engine"
  bash scripts/run_rtl_formal.sh "$engine"
done

for proof in "${shell_proofs[@]}"; do
  printf '\n========== shell formal: %s ==========\n' "${proof%% *}"
  # shellcheck disable=SC2086
  bash scripts/run_rtl_shell_formal.sh $proof
done

# Minimal-viable-system whole-chip proofs (mediation, bounded progress).
mvs_proofs=(
  mediation
  progress
  noninterference
  revocation
)
for proof in "${mvs_proofs[@]}"; do
  printf '\n========== mvs formal: %s ==========\n' "$proof"
  bash scripts/run_rtl_mvs_formal.sh "$proof"
done

printf '%s\n' "rtl formal gates ok"

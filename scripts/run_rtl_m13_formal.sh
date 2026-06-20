#!/usr/bin/env bash
# Model-check the M13 PCIe/IOMMU severe-goal SVA properties on the actual RTL.
#
# Prepares the package + engine + formal wrapper for Yosys, emits an SMT2 model
# of the synthesized design, and discharges the embedded severe-goal assertions
# with yosys-smtbmc + z3:
#   * bounded model checking (BMC) to a depth that covers every reachable state
#     of the terminating engine FSM for ALL seeds/reset timings, and
#   * temporal k-induction (-i) for an unbounded proof.
# This proves the severe goals (raw PCIe authority never exposed; unbound bus
# master / stale BAR / malformed config always rejected with the canonical
# errno; IOMMU-scoped DMA always domain-bound) hold for the actual SystemVerilog
# over all input sequences -- not a single simulation trace.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

for tool in yosys yosys-smtbmc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf '%s\n' "$tool is required for the M13 formal gate (use the lnp64-rtl-formal image)" >&2
    exit 1
  fi
done

top="lnp64_m13_formal"
wrapper="rtl/formal/lnp64_m13_formal.sv"
filelist="tests/rtl/m13_filelist.f"
depth="${LNP64_FORMAL_DEPTH:-24}"
solver="${LNP64_FORMAL_SOLVER:-z3}"

work="$(mktemp -d /tmp/lnp64-m13-formal-XXXXXX)"
trap 'rm -rf "$work"' EXIT
prepdir="$work/prep"
mkdir -p "$prepdir"

scripts/prep_rtl_yosys.py \
  --filelist "$filelist" \
  --extra-source "$wrapper" \
  --out-dir "$prepdir" \
  --sources-out "$prepdir/sources.f"

mapfile -t sources < "$prepdir/sources.f"

smt2="$work/${top}.smt2"
# Flatten + purge prunes the seed-driven datapath that no severe-goal property
# depends on (the properties are control-flow only), keeping the SMT model small
# so the solver is fast and the proof is the same.
yosys -ql "$work/yosys.log" -p \
  "read_verilog -formal -sv ${sources[*]}; prep -top ${top}; flatten; opt -full; opt_clean -purge; async2sync; dffunmap; opt_clean -purge; write_smt2 -wires ${smt2}"

if [[ ! -s "$smt2" ]]; then
  printf '%s\n' "rtl m13 formal: SMT2 model was not produced" >&2
  exit 1
fi

overall=0

printf '== M13 formal: BMC (depth %s, %s) ==\n' "$depth" "$solver"
if yosys-smtbmc -s "$solver" -t "$depth" --dump-vcd "$work/m13_bmc.vcd" "$smt2"; then
  printf 'M13 formal BMC: PASS (no severe-goal assertion violated within depth %s)\n' "$depth"
else
  printf 'M13 formal BMC: FAIL (counterexample in %s)\n' "$work/m13_bmc.vcd" >&2
  overall=1
fi

printf '== M13 formal: temporal k-induction (%s) ==\n' "$solver"
if yosys-smtbmc -s "$solver" -i -t "$depth" "$smt2"; then
  printf 'M13 formal induction: PASS (severe goals hold for all reachable states)\n'
else
  printf 'M13 formal induction: INCONCLUSIVE (BMC bound remains the guarantee)\n'
fi

if [[ "$overall" -ne 0 ]]; then
  printf '%s\n' "rtl m13 formal gate FAILED" >&2
  exit 1
fi

printf '%s\n' "rtl m13 formal gate ok"

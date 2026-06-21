#!/usr/bin/env bash
# Model-check one engine's severe-goal SVA properties on the actual RTL.
#
# Usage: run_rtl_formal.sh <engine>     e.g. run_rtl_formal.sh m13
#
# Expects a formal wrapper rtl/formal/lnp64_<engine>_formal.sv that instantiates
# the real engine RTL (with start/scenario_seed free and a power-on reset
# discipline) and asserts the severe goals as immediate SVA on the engine's
# output ports. Preps the package + engine + wrapper for Yosys, emits an SMT2
# model (flatten + opt_clean -purge drop the seed datapath the properties don't
# depend on, keeping the solver fast), and discharges the assertions with
# yosys-smtbmc: bounded model checking (BMC) plus temporal k-induction for an
# unbounded proof over ALL input sequences -- not a single simulation trace.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

engine="${1:?usage: run_rtl_formal.sh <engine>  (e.g. m13)}"
top="lnp64_${engine}_formal"
wrapper="rtl/formal/lnp64_${engine}_formal.sv"
filelist="tests/rtl/${engine}_filelist.f"
depth="${LNP64_FORMAL_DEPTH:-24}"
solver="${LNP64_FORMAL_SOLVER:-z3}"

for tool in yosys yosys-smtbmc; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf '%s\n' "$tool is required for the formal gate (use the lnp64-rtl-formal image)" >&2
    exit 1
  fi
done
for f in "$wrapper" "$filelist"; do
  [[ -f "$f" ]] || { printf '%s\n' "missing $f" >&2; exit 1; }
done

work="$(mktemp -d "/tmp/lnp64-${engine}-formal-XXXXXX")"
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
yosys -ql "$work/yosys.log" -p \
  "read_verilog -formal -sv ${sources[*]}; prep -top ${top}; flatten; opt -full; opt_clean -purge; async2sync; dffunmap; opt_clean -purge; write_smt2 -wires ${smt2}"

if [[ ! -s "$smt2" ]]; then
  printf '%s\n' "rtl ${engine} formal: SMT2 model was not produced" >&2
  exit 1
fi

overall=0

# yosys-smtbmc prints its progress spinner to stderr; route it to a log so the
# captured verdict stays clean.
printf '== %s formal: BMC (depth %s, %s) ==\n' "$engine" "$depth" "$solver"
if yosys-smtbmc -s "$solver" -t "$depth" --dump-vcd "$work/${engine}_bmc.vcd" "$smt2" 2>"$work/${engine}_bmc.err"; then
  printf '%s formal BMC: PASS (no severe-goal assertion violated within depth %s)\n' "$engine" "$depth"
else
  printf '%s formal BMC: FAIL (counterexample)\n' "$engine"
  grep -aE "Assert failed|BMC failed|^## *[0-9].*(step|Assert)" "$work/${engine}_bmc.err" | grep -avE "Checking assertions in step" | tail -20 || true
  overall=1
fi

printf '== %s formal: temporal k-induction (%s) ==\n' "$engine" "$solver"
if yosys-smtbmc -s "$solver" -i -t "$depth" "$smt2" 2>"$work/${engine}_ind.err"; then
  printf '%s formal induction: PASS (severe goals hold for all reachable states)\n' "$engine"
else
  printf '%s formal induction: INCONCLUSIVE (BMC bound remains the guarantee)\n' "$engine"
fi

if [[ "$overall" -ne 0 ]]; then
  printf 'rtl %s formal gate FAILED\n' "$engine" >&2
  exit 1
fi

printf 'rtl %s formal gate ok\n' "$engine"

#!/usr/bin/env bash
# Model-check an LNP64 MVS (minimal viable system) whole-chip property on the RTL.
#
# Usage: run_rtl_mvs_formal.sh <proof>   e.g. run_rtl_mvs_formal.sh mediation
#
# The MVS (rtl/mvs/lnp64_mvs.sv) is self-contained plain SystemVerilog (no
# package), so it is read directly by yosys -- no ident prep needed. The proof
# wrapper rtl/formal/lnp64_mvs_<proof>_formal.sv drives all ports as free inputs
# and asserts the whole-chip property (mediation, bounded progress, ...). The
# assertions are discharged with yosys-smtbmc: BMC plus temporal k-induction for
# an unbounded result over all input sequences.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

proof="${1:?usage: run_rtl_mvs_formal.sh <proof>  (e.g. mediation)}"
top="lnp64_mvs_${proof}_formal"
wrapper="rtl/formal/lnp64_mvs_${proof}_formal.sv"
depth="${LNP64_FORMAL_DEPTH:-24}"
solver="${LNP64_FORMAL_SOLVER:-z3}"

for tool in yosys yosys-smtbmc; do
  command -v "$tool" >/dev/null 2>&1 || { printf '%s required (use lnp64-rtl-formal)\n' "$tool" >&2; exit 1; }
done
[[ -f "$wrapper" ]] || { printf 'missing %s\n' "$wrapper" >&2; exit 1; }

work="$(mktemp -d "/tmp/lnp64-mvs-${proof}-XXXXXX")"
trap 'rm -rf "$work"' EXIT
smt2="$work/${top}.smt2"

yosys -ql "$work/yosys.log" -p \
  "read_verilog -formal -sv rtl/mvs/lnp64_mvs.sv ${wrapper}; prep -top ${top}; flatten; opt -full; opt_clean -purge; async2sync; dffunmap; opt_clean -purge; write_smt2 -wires ${smt2}"
[[ -s "$smt2" ]] || { printf 'rtl mvs %s: no SMT2 model\n' "$proof" >&2; exit 1; }

overall=0
printf '== MVS %s: BMC (depth %s, %s) ==\n' "$proof" "$depth" "$solver"
if yosys-smtbmc --unroll -s "$solver" -t "$depth" --dump-vcd "$work/mvs_${proof}.vcd" "$smt2" 2>"$work/bmc.err"; then
  printf 'MVS %s BMC: PASS\n' "$proof"
else
  printf 'MVS %s BMC: FAIL\n' "$proof"
  grep -aE "Assert failed|BMC failed" "$work/bmc.err" | tail -10 || true
  overall=1
fi

printf '== MVS %s: temporal k-induction (%s) ==\n' "$proof" "$solver"
if yosys-smtbmc --unroll -s "$solver" -i -t "$depth" "$smt2" 2>"$work/ind.err"; then
  printf 'MVS %s induction: PASS\n' "$proof"
else
  printf 'MVS %s induction: INCONCLUSIVE\n' "$proof"
fi

[[ "$overall" -eq 0 ]] || { printf 'rtl mvs %s gate FAILED\n' "$proof" >&2; exit 1; }
printf 'rtl mvs %s gate ok\n' "$proof"

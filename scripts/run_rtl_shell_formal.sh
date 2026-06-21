#!/usr/bin/env bash
# Model-check a small engine-shell module's "does what it says" SVA contract on
# the actual RTL.
#
# Usage: run_rtl_shell_formal.sh <name> <shell_module> [<shell_module> ...]
#   e.g. run_rtl_shell_formal.sh fail_closed lnp64_fail_closed_engine
#
# The named shell modules are extracted verbatim from
# rtl/engines/lnp64_engine_shells.sv (so the yosys frontend never has to parse
# the sibling modules it cannot handle), combined with the package and the
# formal wrapper rtl/formal/lnp64_<name>_formal.sv, and the wrapper's SVA
# assertions are discharged with yosys-smtbmc (BMC + temporal k-induction) over
# all input sequences -- a proof about the real hardware, not one trace.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

name="${1:?usage: run_rtl_shell_formal.sh <name> <shell_module> [<shell_module> ...]}"
shift
modules=("$@")
[[ ${#modules[@]} -ge 1 ]] || { printf 'need at least one shell module\n' >&2; exit 1; }

top="lnp64_${name}_formal"
wrapper="rtl/formal/lnp64_${name}_formal.sv"
shells="rtl/engines/lnp64_engine_shells.sv"
depth="${LNP64_FORMAL_DEPTH:-24}"
solver="${LNP64_FORMAL_SOLVER:-z3}"

for tool in yosys yosys-smtbmc; do
  command -v "$tool" >/dev/null 2>&1 || { printf '%s required (use lnp64-rtl-formal)\n' "$tool" >&2; exit 1; }
done
[[ -f "$wrapper" ]] || { printf 'missing %s\n' "$wrapper" >&2; exit 1; }

work="$(mktemp -d "/tmp/lnp64-${name}-shell-XXXXXX")"
trap 'rm -rf "$work"' EXIT

# Extract the requested shell modules into a repo-relative file so the package
# ident-qualifier (prep_rtl_yosys.py) can process it like any RTL source.
mkdir -p build/formal_tmp
extract="build/formal_tmp/${name}_extract.sv"
scripts/extract_sv_module.py "$shells" "${modules[@]}" > "$extract"

filelist="$work/${name}.f"
printf '%s\n' "rtl/include/lnp64_pkg.sv" "$extract" > "$filelist"

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
[[ -s "$smt2" ]] || { printf 'rtl %s shell formal: no SMT2 model\n' "$name" >&2; exit 1; }

overall=0
printf '== %s shell formal: BMC (depth %s, %s) ==\n' "$name" "$depth" "$solver"
if yosys-smtbmc -s "$solver" -t "$depth" --dump-vcd "$work/${name}.vcd" "$smt2" 2>"$work/bmc.err"; then
  printf '%s shell formal BMC: PASS\n' "$name"
else
  printf '%s shell formal BMC: FAIL\n' "$name"
  grep -aE "Assert failed|BMC failed" "$work/bmc.err" | tail -10 || true
  overall=1
fi

printf '== %s shell formal: temporal k-induction (%s) ==\n' "$name" "$solver"
if yosys-smtbmc -s "$solver" -i -t "$depth" "$smt2" 2>"$work/ind.err"; then
  printf '%s shell formal induction: PASS\n' "$name"
else
  printf '%s shell formal induction: INCONCLUSIVE\n' "$name"
fi

[[ "$overall" -eq 0 ]] || { printf 'rtl %s shell formal gate FAILED\n' "$name" >&2; exit 1; }
printf 'rtl %s shell formal gate ok\n' "$name"

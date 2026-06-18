#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' "python3 is required for the RTL Yosys S0 gate" >&2
  exit 1
fi

if ! command -v yosys >/dev/null 2>&1; then
  printf '%s\n' "yosys is required for the RTL Yosys S0 gate" >&2
  exit 1
fi

tmpdir="$(mktemp -d /tmp/lnp64-yosys-s0-XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT

scripts/prep_rtl_yosys.py \
  --filelist tests/rtl/s0_filelist.f \
  --out-dir "$tmpdir" \
  --sources-out "$tmpdir/sources.f"

mapfile -t yosys_sources < "$tmpdir/sources.f"

netlist_out="${LNP64_YOSYS_NETLIST_OUT:-$tmpdir/lnp64_s0_yosys_netlist.v}"
yosys -q -p "read_verilog -sv ${yosys_sources[*]}; hierarchy -check -top lnp64_top; synth -top lnp64_top; check; write_verilog -noattr $netlist_out"

if [ ! -s "$netlist_out" ]; then
  printf '%s\n' "rtl yosys s0 failed: netlist output is empty" >&2
  exit 1
fi

printf '%s\n' "rtl yosys s0 ok"

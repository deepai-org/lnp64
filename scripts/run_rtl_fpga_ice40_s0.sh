#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' "python3 is required for the RTL FPGA iCE40 S0 gate" >&2
  exit 1
fi

if ! command -v yosys >/dev/null 2>&1; then
  printf '%s\n' "yosys is required for the RTL FPGA iCE40 S0 gate" >&2
  exit 1
fi

if ! command -v nextpnr-ice40 >/dev/null 2>&1; then
  printf '%s\n' "nextpnr-ice40 is required for the RTL FPGA iCE40 S0 gate" >&2
  exit 1
fi

if ! command -v icepack >/dev/null 2>&1; then
  printf '%s\n' "icepack is required for the RTL FPGA iCE40 S0 gate" >&2
  exit 1
fi

if ! command -v icetime >/dev/null 2>&1; then
  printf '%s\n' "icetime is required for the RTL FPGA iCE40 S0 gate" >&2
  exit 1
fi

tmpdir="$(mktemp -d /tmp/lnp64-fpga-ice40-s0-XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT

scripts/prep_rtl_yosys.py \
  --filelist tests/rtl/s0_fpga_filelist.f \
  --out-dir "$tmpdir" \
  --sources-out "$tmpdir/sources.f"

mapfile -t yosys_sources < "$tmpdir/sources.f"

json_out="${LNP64_ICE40_JSON_OUT:-$tmpdir/lnp64_s0_ice40.json}"
asc_out="${LNP64_ICE40_ASC_OUT:-$tmpdir/lnp64_s0_ice40.asc}"
bin_out="${LNP64_ICE40_BIN_OUT:-$tmpdir/lnp64_s0_ice40.bin}"
report_out="${LNP64_ICE40_REPORT_OUT:-$tmpdir/lnp64_s0_ice40.rpt}"
icetime_summary_out="${LNP64_ICETIME_SUMMARY_OUT:-$tmpdir/lnp64_s0_icetime.summary}"
icetime_report_out="${LNP64_ICETIME_REPORT_OUT:-$tmpdir/lnp64_s0_icetime.rpt}"
icetime_json_out="${LNP64_ICETIME_JSON_OUT:-$tmpdir/lnp64_s0_icetime.json}"
target_freq_mhz="${LNP64_ICE40_TARGET_FREQ_MHZ:-12}"
pcf="${LNP64_ICE40_PCF:-fpga/constraints/lnp64_s0_ice40_hx8k_ct256.pcf}"

if [ ! -s "$pcf" ]; then
  printf 'RTL FPGA iCE40 S0 gate failed: missing PCF %s\n' "$pcf" >&2
  exit 1
fi

yosys -q -p "read_verilog -sv ${yosys_sources[*]}; hierarchy -check -top lnp64_s0_fpga_top; synth_ice40 -device hx -top lnp64_s0_fpga_top -json $json_out"
nextpnr-ice40 \
  --hx8k \
  --package ct256 \
  --json "$json_out" \
  --asc "$asc_out" \
  --freq "$target_freq_mhz" \
  --pcf "$pcf" \
  --report "$report_out" \
  --quiet
icepack "$asc_out" "$bin_out"
icetime \
  -d hx8k \
  -P ct256 \
  -p "$pcf" \
  -c "$target_freq_mhz" \
  -t \
  -r "$icetime_report_out" \
  -j "$icetime_json_out" \
  "$asc_out" > "$icetime_summary_out"

if [ ! -s "$bin_out" ]; then
  printf '%s\n' "RTL FPGA iCE40 S0 gate failed: bitstream output is empty" >&2
  exit 1
fi

if [ ! -s "$report_out" ]; then
  printf '%s\n' "RTL FPGA iCE40 S0 gate failed: nextpnr report output is empty" >&2
  exit 1
fi

if [ ! -s "$icetime_summary_out" ]; then
  printf '%s\n' "RTL FPGA iCE40 S0 gate failed: icetime summary output is empty" >&2
  exit 1
fi

if [ ! -s "$icetime_report_out" ]; then
  printf '%s\n' "RTL FPGA iCE40 S0 gate failed: icetime report output is empty" >&2
  exit 1
fi

if [ ! -s "$icetime_json_out" ]; then
  printf '%s\n' "RTL FPGA iCE40 S0 gate failed: icetime JSON output is empty" >&2
  exit 1
fi

scripts/check_ice40_report.py \
  --report "$report_out" \
  --min-frequency-mhz "$target_freq_mhz"

scripts/check_icetime_report.py \
  --summary "$icetime_summary_out" \
  --min-frequency-mhz "$target_freq_mhz"

printf '%s\n' "rtl fpga ice40 s0 bitstream ok"

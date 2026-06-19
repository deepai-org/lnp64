#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

source scripts/rtl_verilator_common.sh

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL FPGA UART S0 gate" >&2
  exit 1
fi

common_flags=(
  --timing
  -sv
  -Wall
  -Wno-fatal
  -Wno-DECLFILENAME
  -Wno-PINCONNECTEMPTY
  -Wno-TIMESCALEMOD
  -Wno-IMPORTSTAR
  -Wno-WIDTH
  -Wno-UNUSEDSIGNAL
  -Wno-UNUSEDPARAM
  -Wno-BLKSEQ
  --top-module lnp64_s0_fpga_tb
)

mapfile -t rtl_files < tests/rtl/s0_fpga_uart_filelist.f

build_dir="$(rtl_build_dir "s0_fpga")"
rtl_prepare_build_dir "$build_dir"

rtl_lint "${common_flags[@]}" "${rtl_files[@]}"
verilator --binary --Mdir "$build_dir" "${common_flags[@]}" "${rtl_files[@]}" >/tmp/lnp64_rtl_s0_fpga_build.log
"$build_dir/Vlnp64_s0_fpga_tb" | tee /tmp/lnp64_rtl_s0_fpga_sim.log

grep -q "LNP64-RTL-S0-FPGA PASS" /tmp/lnp64_rtl_s0_fpga_sim.log
printf '%s\n' "rtl fpga uart s0 gate ok"

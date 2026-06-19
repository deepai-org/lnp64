#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

source scripts/rtl_verilator_common.sh

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL S0 gate" >&2
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
  --top-module lnp64_s0_tb
)

mapfile -t rtl_files < tests/rtl/s0_filelist.f

build_dir="$(rtl_build_dir "s0")"
rtl_binary="$build_dir/Vlnp64_s0_tb"

rtl_verilator_build_or_reuse \
  "$build_dir" \
  "$rtl_binary" \
  "/tmp/lnp64_rtl_s0_build.log" \
  "${common_flags[@]}" \
  "${rtl_files[@]}"
"$rtl_binary" | tee /tmp/lnp64_rtl_s0_sim.log

grep -q "LNP64-RTL-S0 PASS" /tmp/lnp64_rtl_s0_sim.log
printf '%s\n' "rtl s0 gate ok"

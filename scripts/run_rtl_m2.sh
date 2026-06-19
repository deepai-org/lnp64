#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

source scripts/rtl_verilator_common.sh

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL M2 gate" >&2
  exit 1
fi

common_flags=(
  --timing
  -sv
  -Wall
  -Wno-fatal
  -Wno-DECLFILENAME
  -Wno-TIMESCALEMOD
  -Wno-IMPORTSTAR
  -Wno-UNUSEDSIGNAL
  -Wno-UNUSEDPARAM
  -Wno-BLKSEQ
  -Wno-WIDTH
  -Wno-SYNCASYNCNET
  --top-module lnp64_m2_tb
)

mapfile -t rtl_files < tests/rtl/m2_filelist.f

build_dir="$(rtl_build_dir "m2")"
rtl_binary="$build_dir/Vlnp64_m2_tb"
seeds="${LNP64_COSIM_SEEDS:-0}"

rtl_verilator_build_or_reuse \
  "$build_dir" \
  "$rtl_binary" \
  "/tmp/lnp64_rtl_m2_build.log" \
  "${common_flags[@]}" \
  "${rtl_files[@]}"

rtl_run_seeded_trace_cosim "m2" "$rtl_binary" "formal/m2_gate_model.py" "LNP64-RTL-M2 PASS" "$seeds"

printf '%s\n' "rtl m2 gate ok"

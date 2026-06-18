#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL M13 gate" >&2
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
  --top-module lnp64_m13_tb
)

mapfile -t rtl_files < tests/rtl/m13_filelist.f

build_dir="${TMPDIR:-/tmp}/lnp64_rtl_m13_obj"
seeds="${LNP64_COSIM_SEEDS:-0}"
rm -rf "$build_dir"

verilator --lint-only "${common_flags[@]}" "${rtl_files[@]}"
verilator --binary --Mdir "$build_dir" "${common_flags[@]}" "${rtl_files[@]}" >/tmp/lnp64_rtl_m13_build.log

for seed in $seeds; do
  model_trace="${TMPDIR:-/tmp}/lnp64_rtl_m13_model_${seed}.trace"
  rtl_log="${TMPDIR:-/tmp}/lnp64_rtl_m13_sim_${seed}.log"
  rtl_trace="${TMPDIR:-/tmp}/lnp64_rtl_m13_rtl_${seed}.trace"
  LNP64_COSIM_SEED="$seed" formal/m13_pcie_iommu_model.py > "$model_trace"
  "$build_dir/Vlnp64_m13_tb" "+seed=$seed" | tee "$rtl_log"
  grep '^TRACE ' "$rtl_log" > "$rtl_trace"
  diff -u "$model_trace" "$rtl_trace"
  grep -q "LNP64-RTL-M13 PASS" "$rtl_log"
done

printf '%s\n' "rtl m13 gate ok"

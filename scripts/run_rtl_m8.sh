#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

source scripts/rtl_verilator_common.sh

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL M8 gate" >&2
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
  --top-module lnp64_m8_tb
)

mapfile -t rtl_files < tests/rtl/m8_filelist.f

build_dir="$(rtl_build_dir "m8")"
rtl_binary="$build_dir/Vlnp64_m8_tb"
seeds="${LNP64_COSIM_SEEDS:-0}"

rtl_verilator_build_or_reuse \
  "$build_dir" \
  "$rtl_binary" \
  "/tmp/lnp64_rtl_m8_build.log" \
  "${common_flags[@]}" \
  "${rtl_files[@]}"

for seed in $seeds; do
  model_trace="${TMPDIR:-/tmp}/lnp64_rtl_m8_model_${seed}.trace"
  rtl_log="${TMPDIR:-/tmp}/lnp64_rtl_m8_sim_${seed}.log"
  rtl_trace="${TMPDIR:-/tmp}/lnp64_rtl_m8_rtl_${seed}.trace"
  LNP64_COSIM_SEED="$seed" formal/m8_heap_model.py > "$model_trace"
  "$rtl_binary" "+seed=$seed" | tee "$rtl_log"
  grep '^TRACE ' "$rtl_log" > "$rtl_trace"
  diff -u "$model_trace" "$rtl_trace"
  grep -q "LNP64-RTL-M8 PASS" "$rtl_log"
done

printf '%s\n' "rtl m8 gate ok"

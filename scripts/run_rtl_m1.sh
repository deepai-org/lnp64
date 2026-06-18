#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL M1 gate" >&2
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
  --top-module lnp64_m1_tb
)

mapfile -t rtl_files < tests/rtl/m1_filelist.f

build_dir="${TMPDIR:-/tmp}/lnp64_rtl_m1_obj"
model_trace="${TMPDIR:-/tmp}/lnp64_rtl_m1_model.trace"
rtl_log="${TMPDIR:-/tmp}/lnp64_rtl_m1_sim.log"
rtl_trace="${TMPDIR:-/tmp}/lnp64_rtl_m1_rtl.trace"
rm -rf "$build_dir"

formal/m1_model.py > "$model_trace"
verilator --lint-only "${common_flags[@]}" "${rtl_files[@]}"
verilator --binary --Mdir "$build_dir" "${common_flags[@]}" "${rtl_files[@]}" >/tmp/lnp64_rtl_m1_build.log
"$build_dir/Vlnp64_m1_tb" | tee "$rtl_log"
grep '^TRACE ' "$rtl_log" > "$rtl_trace"
diff -u "$model_trace" "$rtl_trace"
grep -q "LNP64-RTL-M1 PASS" "$rtl_log"
printf '%s\n' "rtl m1 gate ok"

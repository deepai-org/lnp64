#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' "python3 is required for the RTL synthesis smoke gate" >&2
  exit 1
fi

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL synthesis smoke gate" >&2
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
  -Wno-UNUSEDSIGNAL
  -Wno-UNUSEDPARAM
  -Wno-BLKSEQ
  -Wno-WIDTH
  -Wno-SYNCASYNCNET
)

lint_top() {
  local name="$1"
  local top="$2"
  local filelist="$3"
  mapfile -t rtl_files < "$filelist"
  verilator --lint-only "${common_flags[@]}" --top-module "$top" "${rtl_files[@]}"
  printf 'rtl synth-smoke %s ok\n' "$name"
}

scripts/check_rtl_synth_constraints.py
scripts/check_fpga_bringup_manifest.py
scripts/check_rtl_track_b_manifest.py
scripts/check_rtl_s0_contract.py
scripts/check_formal_rtl_roadmap_audit.py
bash scripts/run_rtl_yosys_s0.sh
bash scripts/run_rtl_yosys_vertical_slices.sh

lint_top s0 lnp64_top tests/rtl/s0_filelist.f
lint_top m1 lnp64_m1_pingpong tests/rtl/m1_filelist.f
lint_top m2 lnp64_m2_gate tests/rtl/m2_filelist.f
lint_top m3 lnp64_m3_process tests/rtl/m3_filelist.f
lint_top m4 lnp64_m4_vma tests/rtl/m4_filelist.f
lint_top m5 lnp64_m5_dma tests/rtl/m5_filelist.f
lint_top m6 lnp64_m6_service tests/rtl/m6_filelist.f
lint_top m7 lnp64_m7_futex_atomic tests/rtl/m7_filelist.f
lint_top m8 lnp64_m8_heap tests/rtl/m8_filelist.f
lint_top m9 lnp64_m9_classifier_servicelet tests/rtl/m9_filelist.f
lint_top m10 lnp64_m10_ras tests/rtl/m10_filelist.f
lint_top m11 lnp64_m11_ddr_metadata tests/rtl/m11_filelist.f
lint_top m12 lnp64_m12_storage_barrier tests/rtl/m12_filelist.f
lint_top m13 lnp64_m13_pcie_iommu tests/rtl/m13_filelist.f
lint_top m14 lnp64_m14_resource_domain_policy tests/rtl/m14_filelist.f
lint_top m15 lnp64_m15_object_profiles tests/rtl/m15_filelist.f

printf '%s\n' "rtl synthesis smoke ok"

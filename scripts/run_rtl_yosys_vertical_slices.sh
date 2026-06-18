#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' "python3 is required for the RTL Yosys vertical-slice gate" >&2
  exit 1
fi

if ! command -v yosys >/dev/null 2>&1; then
  printf '%s\n' "yosys is required for the RTL Yosys vertical-slice gate" >&2
  exit 1
fi

tmpdir="$(mktemp -d /tmp/lnp64-yosys-vertical-XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT

tops=(
  "m1:lnp64_m1_pingpong:tests/rtl/m1_filelist.f"
  "m2:lnp64_m2_gate:tests/rtl/m2_filelist.f"
  "m3:lnp64_m3_process:tests/rtl/m3_filelist.f"
  "m4:lnp64_m4_vma:tests/rtl/m4_filelist.f"
  "m5:lnp64_m5_dma:tests/rtl/m5_filelist.f"
  "m6:lnp64_m6_service:tests/rtl/m6_filelist.f"
  "m7:lnp64_m7_futex_atomic:tests/rtl/m7_filelist.f"
  "m8:lnp64_m8_heap:tests/rtl/m8_filelist.f"
  "m9:lnp64_m9_classifier_servicelet:tests/rtl/m9_filelist.f"
  "m10:lnp64_m10_ras:tests/rtl/m10_filelist.f"
  "m11:lnp64_m11_ddr_metadata:tests/rtl/m11_filelist.f"
  "m12:lnp64_m12_storage_barrier:tests/rtl/m12_filelist.f"
  "m13:lnp64_m13_pcie_iommu:tests/rtl/m13_filelist.f"
  "m14:lnp64_m14_resource_domain_policy:tests/rtl/m14_filelist.f"
  "m15:lnp64_m15_object_profiles:tests/rtl/m15_filelist.f"
)

for spec in "${tops[@]}"; do
  IFS=: read -r name top filelist <<< "$spec"
  slice_dir="$tmpdir/$name"
  mkdir -p "$slice_dir"

  scripts/prep_rtl_yosys.py \
    --filelist "$filelist" \
    --out-dir "$slice_dir/src" \
    --sources-out "$slice_dir/sources.f"

  mapfile -t yosys_sources < "$slice_dir/sources.f"
  netlist_out="${LNP64_YOSYS_VERTICAL_OUT_DIR:-$slice_dir}/${top}_netlist.v"

  yosys -q -p "read_verilog -sv ${yosys_sources[*]}; hierarchy -check -top $top; synth -top $top; check; write_verilog -noattr $netlist_out"

  if [ ! -s "$netlist_out" ]; then
    printf 'rtl yosys vertical slice %s failed: netlist output is empty\n' "$name" >&2
    exit 1
  fi
  printf 'rtl yosys vertical slice %s ok\n' "$name"
done

printf '%s\n' "rtl yosys vertical slices ok"

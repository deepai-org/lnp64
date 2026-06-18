#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check_rtl_cosim_manifest.py

export LNP64_COSIM_SEEDS="${LNP64_COSIM_SEEDS:-0 1 7 42 255 1024 4095 4096 65536 1048576 16777216 134217728 268435456 536870912}"
bash scripts/run_rtl_m1.sh
bash scripts/run_rtl_m2.sh
bash scripts/run_rtl_m3.sh
bash scripts/run_rtl_m4.sh
bash scripts/run_rtl_m5.sh
bash scripts/run_rtl_m6.sh
bash scripts/run_rtl_m7.sh
bash scripts/run_rtl_m8.sh
bash scripts/run_rtl_m9.sh
bash scripts/run_rtl_m10.sh
bash scripts/run_rtl_m11.sh
bash scripts/run_rtl_m12.sh
bash scripts/run_rtl_m13.sh
bash scripts/run_rtl_m14.sh
bash scripts/run_rtl_m15.sh

printf '%s\n' "rtl random cosim ok"

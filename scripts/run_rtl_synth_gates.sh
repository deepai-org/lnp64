#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

bash scripts/run_rtl_synth_smoke.sh
bash scripts/run_rtl_fpga_uart_s0.sh
bash scripts/run_rtl_fpga_ice40_s0.sh

printf '%s\n' "rtl synthesis gates ok"

#!/usr/bin/env bash
set -euo pipefail

LNP64_RTL_TOP_PROGRAM_CROSS_TILE_WAKE=1 \
  bash scripts/run_rtl_top_program_smoke.sh "$@"

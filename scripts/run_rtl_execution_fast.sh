#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

export LNP64_RTL_FAST="${LNP64_RTL_FAST:-1}"
export LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}"
export LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}"
export LNP64_RTL_BUILD_ROOT="${LNP64_RTL_BUILD_ROOT:-$root/target/rtl-verilator}"
export LNP64_RTL_TOP_PROGRAM_JOBS="${LNP64_RTL_TOP_PROGRAM_JOBS:-auto}"
export LNP64_RTL_TOP_PROGRAM_MAX_CYCLES="${LNP64_RTL_TOP_PROGRAM_MAX_CYCLES:-10000}"

if [[ -z "${LNP64_BIN:-}" ]]; then
  cargo build --quiet
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    export LNP64_BIN="$CARGO_TARGET_DIR/debug/lnp64"
  else
    export LNP64_BIN="$root/target/debug/lnp64"
  fi
fi

scripts/check_rtl_top_level_program_manifest.py

if [[ "$#" -gt 0 ]]; then
  bash scripts/run_rtl_top_program_manifest.sh "$@"
else
  bash scripts/run_rtl_top_program_manifest.sh
fi

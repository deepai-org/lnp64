#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

program_input="${1:-tests/rtl/programs/top_return_12.c}"
case "$program_input" in
  *.c) ;;
  *)
    printf 'toy C RTL smoke expects a .c input, got: %s\n' "$program_input" >&2
    exit 1
    ;;
esac

LNP64_RTL_TOP_PROGRAM_C_BACKEND=toy \
  bash scripts/run_rtl_top_program_smoke.sh "$@"

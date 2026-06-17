#!/usr/bin/env bash
set -euo pipefail

scripts=(
  scripts/run_inih.sh
  scripts/run_zlib.sh
  scripts/run_natsort.sh
  scripts/run_cwalk.sh
  scripts/run_jsmn.sh
  scripts/run_libc_test.sh
  scripts/run_sbase.sh
)

for script in "${scripts[@]}"; do
  printf '== %s ==\n' "$script"
  bash "$script"
done

printf '%s\n' "real package gates ok"

#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "== real LLVM LNP64 package gate =="
bash scripts/run_real_llvm_package_gate.sh
printf '%s\n' "real package gates ok"

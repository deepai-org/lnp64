#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "== real LLVM LNP64 package gate: zlib =="
LNP64_LLVM_PACKAGE_FILTER=zlib bash scripts/run_real_llvm_package_gate.sh
printf '%s\n' "zlib ok"

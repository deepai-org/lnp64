#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

export LNP64_LLVM_GATE=objects
exec bash scripts/run_real_llvm_lnp64_docker.sh

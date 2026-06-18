#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo fmt --check
cargo test
cargo build --release

export LNP64_BIN="${TMPDIR:-/tmp}/lnp64-gate-bin"
cp target/release/lnp64 "$LNP64_BIN"

bash scripts/run_toolchain_contracts.sh
bash scripts/run_llvm_bootstrap_gates.sh --dry-run
bash scripts/run_netbsd_personality_smoke.sh
bash scripts/run_demos.sh
bash scripts/run_userland.sh
bash scripts/run_netbsd_personality_system.sh
bash scripts/run_real_packages.sh

printf '%s\n' "software gates ok"

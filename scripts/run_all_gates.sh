#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo test

bash scripts/run_netbsd_personality_smoke.sh
bash scripts/run_demos.sh
bash scripts/run_userland.sh
bash scripts/run_netbsd_personality_system.sh
bash scripts/run_real_packages.sh

git diff --check

printf '%s\n' "all gates ok"

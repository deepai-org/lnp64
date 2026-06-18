#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

bash scripts/run_rtl_proof_docker.sh
bash scripts/run_rtl_synth_docker.sh
bash scripts/run_formal_rtl_roadmap_audit.sh
bash scripts/run_software_gates.sh
git diff --check

printf '%s\n' "all gates ok"

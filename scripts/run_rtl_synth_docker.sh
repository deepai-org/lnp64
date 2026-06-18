#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_RTL_SYNTH_IMAGE:-lnp64-rtl-synth}"

docker build \
  -f Dockerfile.rtl-synth \
  -t "$image" \
  .

docker run --rm \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_rtl_synth_gates.sh

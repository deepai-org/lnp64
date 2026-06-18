#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_RTL_PROOF_IMAGE:-lnp64-rtl-proof}"
lean_toolchain="${LNP64_LEAN_TOOLCHAIN:-stable}"

docker build \
  -f Dockerfile.rtl-proof \
  --build-arg "LEAN_TOOLCHAIN=${lean_toolchain}" \
  -t "$image" \
  .

docker run --rm \
  -e LNP64_REQUIRE_LEAN=1 \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_rtl_proof_gates.sh

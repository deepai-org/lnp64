#!/usr/bin/env bash
# Check the LNP64 foundational Coq proofs (the abstract spec the hardware must
# refine). Run inside the lnp64-coq-koika image, which provides native Coq 8.18
# + Koika. coqchk re-validates the compiled proofs in the kernel and reports the
# axioms relied on (must be <none> for a foundational result).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

eval "$(opam env 2>/dev/null)" || true

if ! command -v coqc >/dev/null 2>&1; then
  printf '%s\n' "coqc is required (use the lnp64-coq-koika image)" >&2
  exit 1
fi

coq_files=(
  proofs/coq/CapSpec.v
)

for f in "${coq_files[@]}"; do
  printf '== coqc %s ==\n' "$f"
  coqc "$f"
done

printf '== coqchk (kernel re-check + axiom report) ==\n'
vos=()
for f in "${coq_files[@]}"; do vos+=("${f%.v}.vo"); done
coqchk -silent -o "${vos[@]}"

printf '%s\n' "coq proofs ok"

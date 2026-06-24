#!/usr/bin/env bash
# D2 grep-guard for the ISA collapse (F1/F2). Decode removal alone does not catch
# a stale emitter — an assembler mnemonic that no longer exists only breaks at
# assemble/link/run time. This gate fails loudly if any hand-written source still
# emits an opcode mnemonic that has been retired from the ISA surface, so every
# F1/F2 removal commit can assert "nothing emits the freed form" up front.
#
# Add a mnemonic here the moment its opcode is freed (see the B1 burndown table in
# isa_v2_unification_impl_status.md). A match in a comment (# ... or // ...) is
# ignored; only instruction-position emissions count.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Retired assembler mnemonics (opcode freed → must have no emitters).
retired=(
  # F1-step-1: 0x70/0x72 dynamic waitable_probe/await_ex twins.
  POLL_FD_DYN
  AWAIT_EX_DYN
  WAITABLE_PROBE_DYN
  # F1-step-2: 0x3b/0x3c read_fd/write_fd (=== pull/push) dyn twins.
  READ_FD_DYN
  WRITE_FD_DYN
  PULL_DYN
  PUSH_DYN
)

# Hand-written assembly / C the toolchain assembles or compiles.
search_dirs=(toolchain demos tests/rtl/programs tests/rtl)

status=0
for mnem in "${retired[@]}"; do
  # Instruction position: start of line (after optional whitespace), the mnemonic,
  # then a word boundary. Strip comment lines first so doc references don't trip it.
  hits="$(grep -rn --include='*.s' --include='*.S' --include='*.c' --include='*.h' \
            -E "^[[:space:]]*${mnem}\b" "${search_dirs[@]}" 2>/dev/null \
          | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(#|//)' || true)"
  if [[ -n "$hits" ]]; then
    printf 'retired mnemonic %s still emitted:\n%s\n' "$mnem" "$hits" >&2
    status=1
  fi
done

if [[ "$status" -eq 0 ]]; then
  printf 'retired-mnemonic guard ok (%d mnemonics, no live emitters)\n' "${#retired[@]}"
fi
exit "$status"

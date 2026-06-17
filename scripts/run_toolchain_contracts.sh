#!/usr/bin/env bash
set -euo pipefail

contract_index=toolchain/lnp64_contracts.manifest

while IFS='|' read -r name path test; do
  if [[ -z "$name" || "$name" == \#* ]]; then
    continue
  fi
  if [[ ! -f "$path" ]]; then
    printf 'missing toolchain contract %s at %s\n' "$name" "$path" >&2
    exit 1
  fi
  cargo test --quiet "$test"
done < "$contract_index"

printf '%s\n' "toolchain contracts ok"

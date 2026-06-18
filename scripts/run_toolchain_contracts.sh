#!/usr/bin/env bash
set -euo pipefail

contract_index=toolchain/lnp64_contracts.manifest

run_contract_test() {
  local name="$1"
  local test="$2"
  local output

  if ! output=$(cargo test --quiet "$test" 2>&1); then
    printf '%s\n' "$output"
    return 1
  fi
  if ! grep -Eq 'running [1-9][0-9]* tests?' <<<"$output"; then
    printf 'toolchain contract %s references missing test %s\n' "$name" "$test" >&2
    printf '%s\n' "$output" >&2
    return 1
  fi
  printf '%s\n' "$output"
}

while IFS='|' read -r name path test; do
  if [[ -z "$name" || "$name" == \#* ]]; then
    continue
  fi
  if [[ -z "$path" || -z "$test" ]]; then
    printf 'malformed toolchain contract row: %s|%s|%s\n' "$name" "$path" "$test" >&2
    exit 1
  fi
  if [[ ! -f "$path" ]]; then
    printf 'missing toolchain contract %s at %s\n' "$name" "$path" >&2
    exit 1
  fi
  run_contract_test "$name" "$test"
done < "$contract_index"

printf '%s\n' "toolchain contracts ok"

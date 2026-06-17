#!/usr/bin/env bash
set -euo pipefail

tests=(
  llvm_target_manifest_records_required_backend_contract
  relocation_manifest_matches_object_format_and_target_manifest
  psabi_manifest_records_current_calling_convention_contract
  intrinsic_manifest_matches_target_manifest
  isel_manifest_covers_backend_starting_opcode_groups
  exec_plan_manifest_matches_loader_boundary_contract
)

for test in "${tests[@]}"; do
  cargo test --quiet "$test"
done

printf '%s\n' "toolchain contracts ok"

#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check_rtl_top_level_program_manifest.py >/dev/null

if [[ -z "${LNP64_BIN:-}" ]]; then
  cargo build --quiet
  export LNP64_BIN="$root/target/debug/lnp64"
fi

export LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}"
export LNP64_RTL_BUILD_ROOT="${LNP64_RTL_BUILD_ROOT:-$root/target/rtl-verilator}"
export LNP64_RTL_TOP_PROGRAM_MAX_CYCLES="${LNP64_RTL_TOP_PROGRAM_MAX_CYCLES:-10000}"
first_program_reuse_build="$LNP64_RTL_REUSE_BUILD"

top_program_jobs="${LNP64_RTL_TOP_PROGRAM_JOBS:-}"
if [[ -z "$top_program_jobs" ]]; then
  if [[ "${LNP64_RTL_FAST:-0}" == "1" ]]; then
    top_program_jobs=auto
  else
    top_program_jobs=1
  fi
fi
if [[ "$top_program_jobs" == "auto" ]]; then
  top_program_jobs="$(nproc 2>/dev/null || printf '1')"
fi
if ! [[ "$top_program_jobs" =~ ^[0-9]+$ ]] || (( top_program_jobs < 1 )); then
  printf 'LNP64_RTL_TOP_PROGRAM_JOBS must be a positive integer or auto, got %q\n' "$top_program_jobs" >&2
  exit 1
fi

if [[ "$#" -gt 0 ]]; then
  program_specs=()
  for program in "$@"; do
    program_specs+=("${program}"$'\t'"scripts/run_rtl_top_program_smoke.sh")
  done
else
  mapfile -t program_specs < <(
    python3 - <<'PY'
import json
from pathlib import Path

manifest = json.loads(Path("tests/rtl/top_level_program_manifest.json").read_text(encoding="utf-8"))
for section in ("flat_hex_programs", "llvm_mc_programs", "llvm_clang_programs", "llvm_linked_programs", "compiler_flat_programs", "assembly_programs", "compiler_generated_programs"):
    for entry in manifest[section]:
        if entry["status"] == "active":
            print(f"{entry['source']}\t{entry['rtl_gate']}")
PY
  )
fi

if [[ "${#program_specs[@]}" -eq 0 ]]; then
  printf '%s\n' "no active top-level RTL programs selected" >&2
  exit 1
fi

run_program_spec() {
  local spec="$1"
  local program gate
  IFS=$'\t' read -r program gate <<< "$spec"
  if [[ -z "$program" || -z "$gate" ]]; then
    printf 'invalid top-level RTL program spec: %q\n' "$spec" >&2
    exit 1
  fi
  if [[ ! -x "$gate" && ! -f "$gate" ]]; then
    printf 'missing top-level RTL gate: %s\n' "$gate" >&2
    exit 1
  fi
  bash "$gate" "$program"
}

run_reused_program_spec() {
  local spec="$1"
  LNP64_RTL_REUSE_BUILD=1 \
  LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}" \
  LNP64_RTL_TOP_PROGRAM_SKIP_BUILD="${LNP64_RTL_TOP_PROGRAM_SKIP_BUILD:-1}" \
    run_program_spec "$spec"
}

first=1
if (( top_program_jobs == 1 || ${#program_specs[@]} == 1 )); then
  for spec in "${program_specs[@]}"; do
    IFS=$'\t' read -r program gate <<< "$spec"
    printf '\n==> top-level RTL program: %s\n' "$program"
    if [[ "$first" -eq 1 ]]; then
      first=0
      LNP64_RTL_REUSE_BUILD="$first_program_reuse_build" \
        run_program_spec "$spec"
    else
      run_reused_program_spec "$spec"
    fi
  done
else
  first_spec="${program_specs[0]}"
  IFS=$'\t' read -r first_program first_gate <<< "$first_spec"
  printf '\n==> top-level RTL program: %s\n' "$first_program"
  LNP64_RTL_REUSE_BUILD="$first_program_reuse_build" \
    run_program_spec "$first_spec"

  log_dir="$(mktemp -d "${TMPDIR:-/tmp}/lnp64_rtl_top_program_manifest.XXXXXX")"
  cleanup() {
    if [[ "${LNP64_RTL_TOP_PROGRAM_KEEP_LOGS:-0}" != "1" ]]; then
      rm -rf "$log_dir"
    fi
  }
  trap cleanup EXIT

  batch_pids=()
  batch_programs=()
  batch_logs=()
  failed=0

  wait_batch() {
    local i pid program log
    for i in "${!batch_pids[@]}"; do
      pid="${batch_pids[$i]}"
      program="${batch_programs[$i]}"
      log="${batch_logs[$i]}"
      if wait "$pid"; then
        printf 'rtl top-level program %s ok (%s)\n' "$program" "$log"
      else
        failed=1
        printf 'rtl top-level program %s failed (%s)\n' "$program" "$log" >&2
        cat "$log" >&2
      fi
    done
    batch_pids=()
    batch_programs=()
    batch_logs=()
  }

  printf 'running remaining top-level RTL programs with %s parallel job(s); logs in %s\n' "$top_program_jobs" "$log_dir"
  for ((idx = 1; idx < ${#program_specs[@]}; idx++)); do
    spec="${program_specs[$idx]}"
    IFS=$'\t' read -r program gate <<< "$spec"
    log="$log_dir/program_${idx}.log"
    (
      printf '\n==> top-level RTL program: %s\n' "$program"
      run_reused_program_spec "$spec"
    ) >"$log" 2>&1 &
    batch_pids+=("$!")
    batch_programs+=("$program")
    batch_logs+=("$log")
    if (( ${#batch_pids[@]} >= top_program_jobs )); then
      wait_batch
    fi
  done
  wait_batch

  if (( failed != 0 )); then
    exit 1
  fi
fi

printf '\n%s\n' "rtl top-level program manifest gate ok (${#program_specs[@]} programs)"

#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check_rtl_top_level_program_manifest.py >/dev/null

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

mapfile -t program_specs < <(
  python3 - "$@" <<'PY'
import json
import fnmatch
import os
import sys
from pathlib import Path

manifest = json.loads(Path("tests/rtl/top_level_program_manifest.json").read_text(encoding="utf-8"))
entries = []
for section in ("flat_hex_programs", "llvm_mc_programs", "llvm_clang_programs", "llvm_linked_programs", "assembly_programs"):
    for entry in manifest[section]:
        if entry["status"] == "active":
            entries.append(entry)

by_source = {entry["source"]: entry for entry in entries}
requested = sys.argv[1:]
if requested:
    for source in requested:
        entry = by_source.get(source)
        gate = entry["rtl_gate"] if entry else "scripts/run_rtl_top_program_smoke.sh"
        print(f"{source}\t{gate}")
else:
    patterns = [
        pattern
        for raw in os.environ.get("LNP64_RTL_TOP_PROGRAM_FILTER", "").replace(",", " ").split()
        for pattern in [raw.strip()]
        if pattern
    ]
    selected = entries
    if patterns:
        selected = [
            entry
            for entry in entries
            if any(
                fnmatch.fnmatch(entry["source"], pattern) or pattern in entry["source"]
                for pattern in patterns
            )
        ]
    for entry in selected:
        print(f"{entry['source']}\t{entry['rtl_gate']}")
PY
)

if [[ "${#program_specs[@]}" -eq 0 ]]; then
  printf '%s\n' "no active top-level RTL programs selected" >&2
  exit 1
fi

for spec in "${program_specs[@]}"; do
  IFS=$'\t' read -r program gate <<< "$spec"
  if [[ "$program" == *.c && "$gate" == "scripts/run_rtl_top_program_smoke.sh" ]]; then
    printf '%s\n' "direct .c input to run_rtl_top_program_manifest.sh is retired" >&2
    printf '%s\n' "use manifest-owned LLVM clang/linked entries or scripts/run_rtl_top_linked_llvm_smoke.sh for C inputs" >&2
    exit 1
  fi
done

if [[ -z "${LNP64_BIN:-}" ]]; then
  cargo build --quiet
  export LNP64_BIN="$root/target/debug/lnp64"
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

  active_pids=()
  declare -A active_programs=()
  declare -A active_logs=()
  failed=0

  remove_active_pid() {
    local done_pid="$1"
    local remaining=()
    local pid
    for pid in "${active_pids[@]}"; do
      if [[ "$pid" != "$done_pid" ]]; then
        remaining+=("$pid")
      fi
    done
    active_pids=("${remaining[@]}")
  }

  wait_one() {
    local done_pid program log
    if wait -n -p done_pid; then
      program="${active_programs[$done_pid]}"
      log="${active_logs[$done_pid]}"
      printf 'rtl top-level program %s ok (%s)\n' "$program" "$log"
    else
      program="${active_programs[$done_pid]}"
      log="${active_logs[$done_pid]}"
      failed=1
      printf 'rtl top-level program %s failed (%s)\n' "$program" "$log" >&2
      cat "$log" >&2
    fi
    unset "active_programs[$done_pid]" "active_logs[$done_pid]"
    remove_active_pid "$done_pid"
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
    pid="$!"
    active_pids+=("$pid")
    active_programs[$pid]="$program"
    active_logs[$pid]="$log"
    if (( ${#active_pids[@]} >= top_program_jobs )); then
      wait_one
    fi
  done
  while (( ${#active_pids[@]} > 0 )); do
    wait_one
  done

  if (( failed != 0 )); then
    exit 1
  fi
fi

printf '\n%s\n' "rtl top-level program manifest gate ok (${#program_specs[@]} programs)"

#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check_rtl_cosim_manifest.py

export LNP64_COSIM_SEEDS="${LNP64_COSIM_SEEDS:-0 1 7 42 255 1024 4095 4096 65536 1048576 16777216 134217728 268435456 536870912}"

gates=(
  scripts/run_rtl_m1.sh
  scripts/run_rtl_m2.sh
  scripts/run_rtl_m3.sh
  scripts/run_rtl_m4.sh
  scripts/run_rtl_m5.sh
  scripts/run_rtl_m6.sh
  scripts/run_rtl_m7.sh
  scripts/run_rtl_m8.sh
  scripts/run_rtl_m9.sh
  scripts/run_rtl_m10.sh
  scripts/run_rtl_m11.sh
  scripts/run_rtl_m12.sh
  scripts/run_rtl_m13.sh
  scripts/run_rtl_m14.sh
  scripts/run_rtl_m15.sh
)

jobs="${LNP64_RTL_RANDOM_COSIM_JOBS:-1}"
if [[ "$jobs" == "auto" ]]; then
  jobs="$(nproc 2>/dev/null || printf '1')"
fi
if ! [[ "$jobs" =~ ^[0-9]+$ ]] || (( jobs < 1 )); then
  printf 'LNP64_RTL_RANDOM_COSIM_JOBS must be a positive integer or auto, got %q\n' "$jobs" >&2
  exit 1
fi

if (( jobs == 1 )); then
  for gate in "${gates[@]}"; do
    bash "$gate"
  done
else
  log_dir="$(mktemp -d "${TMPDIR:-/tmp}/lnp64_rtl_random_cosim.XXXXXX")"
  cleanup() {
    if [[ "${LNP64_RTL_RANDOM_COSIM_KEEP_LOGS:-0}" != "1" ]]; then
      rm -rf "$log_dir"
    fi
  }
  trap cleanup EXIT

  batch_pids=()
  batch_gates=()
  batch_logs=()
  failed=0

  wait_batch() {
    local i gate label log pid
    for i in "${!batch_pids[@]}"; do
      pid="${batch_pids[$i]}"
      gate="${batch_gates[$i]}"
      label="${gate#scripts/run_rtl_}"
      label="${label%.sh}"
      log="${batch_logs[$i]}"
      if wait "$pid"; then
        printf 'rtl random cosim %s ok (%s)\n' "$label" "$log"
      else
        failed=1
        printf 'rtl random cosim %s failed (%s)\n' "$label" "$log" >&2
        cat "$log" >&2
      fi
    done
    batch_pids=()
    batch_gates=()
    batch_logs=()
  }

  printf 'running rtl random cosim with %s parallel job(s); logs in %s\n' "$jobs" "$log_dir"
  for gate in "${gates[@]}"; do
    label="${gate#scripts/run_rtl_}"
    label="${label%.sh}"
    log="$log_dir/${label}.log"
    bash "$gate" >"$log" 2>&1 &
    batch_pids+=("$!")
    batch_gates+=("$gate")
    batch_logs+=("$log")
    if (( ${#batch_pids[@]} >= jobs )); then
      wait_batch
    fi
  done
  wait_batch

  if (( failed != 0 )); then
    exit 1
  fi
fi

printf '%s\n' "rtl random cosim ok"

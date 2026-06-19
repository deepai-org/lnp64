#!/usr/bin/env bash

rtl_build_dir() {
  local name="$1"
  local build_root="${LNP64_RTL_BUILD_ROOT:-${TMPDIR:-/tmp}}"
  printf '%s/lnp64_rtl_%s_obj\n' "$build_root" "$name"
}

rtl_prepare_build_dir() {
  local build_dir="$1"
  mkdir -p "$(dirname "$build_dir")"
  if [[ "${LNP64_RTL_REUSE_BUILD:-0}" != "1" ]]; then
    rm -rf "$build_dir"
  fi
}

rtl_lock_build_dir() {
  local build_dir="$1"
  mkdir -p "$(dirname "$build_dir")"
  if command -v flock >/dev/null 2>&1; then
    exec {LNP64_RTL_BUILD_LOCK_FD}>"$build_dir.lock"
    flock "$LNP64_RTL_BUILD_LOCK_FD"
  fi
}

rtl_lint() {
  if [[ "${LNP64_RTL_SKIP_LINT:-0}" == "1" ]]; then
    printf '%s\n' "rtl lint-only step skipped (LNP64_RTL_SKIP_LINT=1)"
  else
    verilator --lint-only "$@"
  fi
}

rtl_verilator_build_job_args() {
  local jobs="${LNP64_RTL_VERILATOR_BUILD_JOBS:-}"
  if [[ -z "$jobs" ]]; then
    return
  fi
  if [[ "$jobs" == "auto" ]]; then
    jobs="$(nproc 2>/dev/null || printf '1')"
  fi
  if ! [[ "$jobs" =~ ^[0-9]+$ ]]; then
    printf 'LNP64_RTL_VERILATOR_BUILD_JOBS must be a non-negative integer or auto, got %q\n' "$jobs" >&2
    exit 1
  fi
  printf '%s\n' --build-jobs "$jobs"
}

rtl_verilator_build_or_reuse() {
  local build_dir="$1"
  local binary="$2"
  local build_log="$3"
  shift 3

  if [[ "${LNP64_RTL_SKIP_BUILD:-0}" == "1" ]]; then
    if [[ ! -x "$binary" ]]; then
      printf 'missing reusable RTL binary: %s\n' "$binary" >&2
      printf '%s\n' "unset LNP64_RTL_SKIP_BUILD or run the gate once to build it" >&2
      exit 1
    fi
    printf 'rtl Verilator build skipped: %s\n' "$binary"
    return
  fi

  rtl_prepare_build_dir "$build_dir"
  rtl_lint "$@"
  mapfile -t verilator_build_job_args < <(rtl_verilator_build_job_args)
  verilator --binary --Mdir "$build_dir" "${verilator_build_job_args[@]}" "$@" >"$build_log"
}

rtl_cosim_seed_jobs() {
  local jobs="${LNP64_RTL_COSIM_SEED_JOBS:-1}"
  if [[ "$jobs" == "auto" ]]; then
    jobs="$(nproc 2>/dev/null || printf '1')"
  fi
  if ! [[ "$jobs" =~ ^[0-9]+$ ]] || (( jobs < 1 )); then
    printf 'LNP64_RTL_COSIM_SEED_JOBS must be a positive integer or auto, got %q\n' "$jobs" >&2
    exit 1
  fi
  printf '%s\n' "$jobs"
}

rtl_run_seeded_trace_cosim_one() {
  local tag="$1"
  local rtl_binary="$2"
  local model_program="$3"
  local pass_line="$4"
  local seed="$5"
  local echo_sim="${6:-1}"
  local model_trace="${TMPDIR:-/tmp}/lnp64_rtl_${tag}_model_${seed}.trace"
  local rtl_log="${TMPDIR:-/tmp}/lnp64_rtl_${tag}_sim_${seed}.log"
  local rtl_trace="${TMPDIR:-/tmp}/lnp64_rtl_${tag}_rtl_${seed}.trace"

  LNP64_COSIM_SEED="$seed" "$model_program" > "$model_trace"
  if [[ "$echo_sim" == "1" ]]; then
    "$rtl_binary" "+seed=$seed" | tee "$rtl_log"
  else
    "$rtl_binary" "+seed=$seed" > "$rtl_log"
  fi
  grep '^TRACE ' "$rtl_log" > "$rtl_trace"
  diff -u "$model_trace" "$rtl_trace"
  grep -q "$pass_line" "$rtl_log"
}

rtl_run_seeded_trace_cosim() {
  local tag="$1"
  local rtl_binary="$2"
  local model_program="$3"
  local pass_line="$4"
  local seeds="$5"
  local jobs
  jobs="$(rtl_cosim_seed_jobs)"

  if (( jobs == 1 )); then
    local seed
    for seed in $seeds; do
      rtl_run_seeded_trace_cosim_one "$tag" "$rtl_binary" "$model_program" "$pass_line" "$seed" 1
    done
    return
  fi

  local log_dir
  log_dir="$(mktemp -d "${TMPDIR:-/tmp}/lnp64_rtl_${tag}_seeds.XXXXXX")"
  local batch_pids=()
  local batch_seeds=()
  local batch_logs=()
  local failed=0

  rtl_wait_seed_batch() {
    local i pid seed log
    for i in "${!batch_pids[@]}"; do
      pid="${batch_pids[$i]}"
      seed="${batch_seeds[$i]}"
      log="${batch_logs[$i]}"
      if wait "$pid"; then
        printf 'rtl %s seed %s ok (%s)\n' "$tag" "$seed" "$log"
      else
        failed=1
        printf 'rtl %s seed %s failed (%s)\n' "$tag" "$seed" "$log" >&2
        cat "$log" >&2
      fi
    done
    batch_pids=()
    batch_seeds=()
    batch_logs=()
  }

  printf 'running rtl %s seeds with %s parallel job(s); logs in %s\n' "$tag" "$jobs" "$log_dir"
  local seed log
  for seed in $seeds; do
    log="$log_dir/seed_${seed}.log"
    (
      rtl_run_seeded_trace_cosim_one "$tag" "$rtl_binary" "$model_program" "$pass_line" "$seed" 0
    ) >"$log" 2>&1 &
    batch_pids+=("$!")
    batch_seeds+=("$seed")
    batch_logs+=("$log")
    if (( ${#batch_pids[@]} >= jobs )); then
      rtl_wait_seed_batch
    fi
  done
  rtl_wait_seed_batch

  if [[ "${LNP64_RTL_COSIM_KEEP_SEED_LOGS:-0}" != "1" ]]; then
    rm -rf "$log_dir"
  fi
  if (( failed != 0 )); then
    exit 1
  fi
}

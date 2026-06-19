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
  if [[ -n "${LNP64_RTL_VERILATOR_BUILD_JOBS:-}" ]]; then
    printf '%s\n' --build-jobs "$LNP64_RTL_VERILATOR_BUILD_JOBS"
  fi
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

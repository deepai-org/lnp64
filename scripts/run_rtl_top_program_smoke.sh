#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

source scripts/rtl_verilator_common.sh

tmp_files=()
cleanup() {
  if [[ ${#tmp_files[@]} -gt 0 ]]; then
    rm -f "${tmp_files[@]}"
  fi
}
trap cleanup EXIT

if ! command -v verilator >/dev/null 2>&1; then
  printf '%s\n' "verilator is required for the RTL top-level program smoke gate" >&2
  exit 1
fi

common_flags=(
  --timing
  -sv
  -Wall
  -Wno-fatal
  -Wno-DECLFILENAME
  -Wno-PINCONNECTEMPTY
  -Wno-TIMESCALEMOD
  -Wno-IMPORTSTAR
  -Wno-WIDTH
  -Wno-UNUSEDSIGNAL
  -Wno-UNUSEDPARAM
  -Wno-BLKSEQ
  --top-module lnp64_top_program_tb
)

mapfile -t rtl_files < tests/rtl/top_program_filelist.f

build_dir="$(rtl_build_dir "top_program")"
rtl_lock_build_dir "$build_dir"
rtl_prepare_build_dir "$build_dir"

program_input="${1:-tests/rtl/programs/top_smoke.s}"
if [[ ! -f "$program_input" ]]; then
  printf 'missing top-level program input: %s\n' "$program_input" >&2
  exit 1
fi
program_hex="$program_input"
program_asm=""
program_data_hex=""
if [[ "$program_input" == *.c ]]; then
  program_asm="$(mktemp "${TMPDIR:-/tmp}/lnp64_top_program_from_c.XXXXXX.s")"
  tmp_files+=("$program_asm")
  if [[ -n "${LNP64_BIN:-}" ]]; then
    "$LNP64_BIN" cc --toy-bootstrap "$program_input" -o "$program_asm"
  else
    cargo run --quiet -- cc --toy-bootstrap "$program_input" -o "$program_asm"
  fi
  program_input="$program_asm"
fi
if [[ "$program_input" == *.s ]]; then
  program_hex="$(mktemp "${TMPDIR:-/tmp}/lnp64_top_program_from_asm.XXXXXX.hex")"
  program_data_hex="$(mktemp "${TMPDIR:-/tmp}/lnp64_top_program_data_from_asm.XXXXXX.hex")"
  tmp_files+=("$program_hex" "$program_data_hex")
  if [[ -n "${LNP64_BIN:-}" ]]; then
    "$LNP64_BIN" asm-flat-exec "$program_input" -o "$program_hex" --data-hex "$program_data_hex"
  else
    cargo run --quiet -- asm-flat-exec "$program_input" -o "$program_hex" --data-hex "$program_data_hex"
  fi
fi

sim_log="$(mktemp "${TMPDIR:-/tmp}/lnp64_rtl_top_program_sim.XXXXXX.log")"
emulator_log="$(mktemp "${TMPDIR:-/tmp}/lnp64_emulator_top_program.XXXXXX.log")"
tmp_files+=("$sim_log" "$emulator_log")

rtl_binary="$build_dir/Vlnp64_top_program_tb"
if [[ "${LNP64_RTL_TOP_PROGRAM_SKIP_BUILD:-0}" == "1" ]]; then
  if [[ ! -x "$rtl_binary" ]]; then
    printf 'missing reusable top-level RTL binary: %s\n' "$rtl_binary" >&2
    printf '%s\n' "unset LNP64_RTL_TOP_PROGRAM_SKIP_BUILD or run one build first" >&2
    exit 1
  fi
else
  mapfile -t verilator_build_job_args < <(rtl_verilator_build_job_args)
  rtl_lint "${common_flags[@]}" "${rtl_files[@]}"
  verilator --binary --Mdir "$build_dir" "${verilator_build_job_args[@]}" "${common_flags[@]}" "${rtl_files[@]}" >/tmp/lnp64_rtl_top_program_build.log
fi
if [[ -n "${LNP64_RTL_BUILD_LOCK_FD:-}" ]]; then
  flock -u "$LNP64_RTL_BUILD_LOCK_FD"
fi
rtl_plusargs=("+lnp64_program_hex=$program_hex")
if [[ -n "$program_data_hex" && -s "$program_data_hex" ]]; then
  rtl_plusargs+=("+lnp64_data_hex=$program_data_hex")
fi
if [[ -n "${LNP64_RTL_TOP_PROGRAM_MAX_CYCLES:-}" ]]; then
  rtl_plusargs+=("+lnp64_max_cycles=$LNP64_RTL_TOP_PROGRAM_MAX_CYCLES")
fi
"$rtl_binary" "${rtl_plusargs[@]}" | tee "$sim_log"

grep -q "LNP64-RTL-TOP-PROGRAM PASS" "$sim_log"

if [[ -n "${LNP64_BIN:-}" ]]; then
  emulator_cmd=("$LNP64_BIN" run-flat-exec "$program_hex")
else
  emulator_cmd=(cargo run --quiet -- run-flat-exec "$program_hex")
fi
if [[ -n "$program_data_hex" && -s "$program_data_hex" ]]; then
  emulator_cmd+=(--data-hex "$program_data_hex")
fi
"${emulator_cmd[@]}" | tee "$emulator_log"

python3 - "$sim_log" "$emulator_log" <<'PY'
import json
import sys


def load_record(path: str, prefix: str) -> dict:
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith(prefix):
                return json.loads(line[len(prefix):])
    raise SystemExit(f"missing {prefix.strip()} record in {path}")


def load_records(path: str, prefix: str) -> list[dict]:
    records = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith(prefix):
                records.append(json.loads(line[len(prefix):]))
    if not records:
        raise SystemExit(f"missing {prefix.strip()} records in {path}")
    return records


rtl = load_record(sys.argv[1], "RTL_FINAL ")
emulator = load_record(sys.argv[2], "EMULATOR_FINAL ")
if rtl["exit_reg"] != emulator["exit"]:
    raise SystemExit(f"exit mismatch: rtl={rtl['exit_reg']} emulator={emulator['exit']}")
for field in ("r3", "r4", "r5", "env_page", "mem0"):
    if rtl[field] != emulator[field]:
        raise SystemExit(f"{field} mismatch: rtl={rtl[field]} emulator={emulator[field]}")

rtl_retire = load_records(sys.argv[1], "RTL_RETIRE ")
emulator_retire = load_record(sys.argv[2], "EMULATOR_RETIRE ")
if rtl_retire != emulator_retire:
    limit = min(len(rtl_retire), len(emulator_retire))
    first = next(
        (idx for idx in range(limit) if rtl_retire[idx] != emulator_retire[idx]),
        limit,
    )
    start = max(0, first - 3)
    end = min(max(len(rtl_retire), len(emulator_retire)), first + 4)
    raise SystemExit(
        "retire trace mismatch: "
        f"first_diff={first} rtl_len={len(rtl_retire)} emulator_len={len(emulator_retire)} "
        f"rtl_window={rtl_retire[start:min(end, len(rtl_retire))]} "
        f"emulator_window={emulator_retire[start:min(end, len(emulator_retire))]}"
    )
PY

printf '%s\n' "rtl top-level program smoke ok"

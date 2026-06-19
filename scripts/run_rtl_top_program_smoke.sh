#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

source scripts/rtl_verilator_common.sh

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
rtl_prepare_build_dir "$build_dir"

program_input="${1:-tests/rtl/programs/top_smoke.s}"
if [[ ! -f "$program_input" ]]; then
  printf 'missing top-level program input: %s\n' "$program_input" >&2
  exit 1
fi
program_hex="$program_input"
if [[ "$program_input" == *.s ]]; then
  program_hex="${TMPDIR:-/tmp}/lnp64_top_program_from_asm.hex"
  if [[ -n "${LNP64_BIN:-}" ]]; then
    "$LNP64_BIN" asm-flat-exec "$program_input" -o "$program_hex"
  else
    cargo run --quiet -- asm-flat-exec "$program_input" -o "$program_hex"
  fi
fi

rtl_lint "${common_flags[@]}" "${rtl_files[@]}"
verilator --binary --Mdir "$build_dir" "${common_flags[@]}" "${rtl_files[@]}" >/tmp/lnp64_rtl_top_program_build.log
"$build_dir/Vlnp64_top_program_tb" "+lnp64_program_hex=$program_hex" | tee /tmp/lnp64_rtl_top_program_sim.log

grep -q "LNP64-RTL-TOP-PROGRAM PASS" /tmp/lnp64_rtl_top_program_sim.log
grep -q 'RTL_FINAL {"retired":9,"exit_reg":12,"r3":12,"r4":12,"r5":0,"env_page":4096,"mem0":12}' /tmp/lnp64_rtl_top_program_sim.log

if [[ -n "${LNP64_BIN:-}" ]]; then
  "$LNP64_BIN" run-flat-exec "$program_hex" | tee /tmp/lnp64_emulator_top_program.log
else
  cargo run --quiet -- run-flat-exec "$program_hex" | tee /tmp/lnp64_emulator_top_program.log
fi

python3 - /tmp/lnp64_rtl_top_program_sim.log /tmp/lnp64_emulator_top_program.log <<'PY'
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
    raise SystemExit(f"retire trace mismatch: rtl={rtl_retire} emulator={emulator_retire}")
PY

printf '%s\n' "rtl top-level program smoke ok"

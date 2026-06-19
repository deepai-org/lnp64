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

program_hex="${TMPDIR:-/tmp}/lnp64_top_program.hex"
python3 - "$program_hex" <<'PY'
from pathlib import Path
import sys


def enc_ri(opcode: int, rd: int, imm: int) -> int:
    return (opcode << 24) | ((rd & 0x1f) << 19) | (imm & 0xffff)


def enc_rrr(opcode: int, rd: int, rs1: int, rs2: int) -> int:
    return (
        (opcode << 24)
        | ((rd & 0x1f) << 19)
        | ((rs1 & 0x1f) << 14)
        | ((rs2 & 0x1f) << 9)
    )


def enc_mem(opcode: int, reg_a: int, base: int, imm: int) -> int:
    return (opcode << 24) | ((reg_a & 0x1f) << 19) | ((base & 0x1f) << 14) | (imm & 0x3fff)


def enc_reg(opcode: int, reg: int) -> int:
    return (opcode << 24) | ((reg & 0x1f) << 19)


program = [
    enc_ri(0x01, 1, 7),
    enc_ri(0x01, 2, 5),
    enc_rrr(0x10, 3, 1, 2),
    enc_mem(0x33, 3, 0, 0),
    enc_mem(0x30, 4, 0, 0),
    enc_reg(0x3a, 4),
]

Path(sys.argv[1]).write_text("".join(f"{word:08x}\n" for word in program))
PY

rtl_lint "${common_flags[@]}" "${rtl_files[@]}"
verilator --binary --Mdir "$build_dir" "${common_flags[@]}" "${rtl_files[@]}" >/tmp/lnp64_rtl_top_program_build.log
"$build_dir/Vlnp64_top_program_tb" "+lnp64_program_hex=$program_hex" | tee /tmp/lnp64_rtl_top_program_sim.log

grep -q "LNP64-RTL-TOP-PROGRAM PASS" /tmp/lnp64_rtl_top_program_sim.log
grep -q 'RTL_FINAL {"retired":6,"exit_reg":12,"r3":12,"r4":12,"mem0":12}' /tmp/lnp64_rtl_top_program_sim.log

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


rtl = load_record(sys.argv[1], "RTL_FINAL ")
emulator = load_record(sys.argv[2], "EMULATOR_FINAL ")
if rtl["exit_reg"] != emulator["exit"]:
    raise SystemExit(f"exit mismatch: rtl={rtl['exit_reg']} emulator={emulator['exit']}")
for field in ("r3", "r4", "mem0"):
    if rtl[field] != emulator[field]:
        raise SystemExit(f"{field} mismatch: rtl={rtl[field]} emulator={emulator[field]}")
PY

printf '%s\n' "rtl top-level program smoke ok"

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

top_program_quiet="${LNP64_RTL_TOP_PROGRAM_QUIET:-${LNP64_RTL_FAST:-0}}"

run_top_program_logged() {
  local log="$1"
  shift
  if [[ "$top_program_quiet" == "1" ]]; then
    if ! "$@" >"$log" 2>&1; then
      cat "$log" >&2
      return 1
    fi
  else
    "$@" 2>&1 | tee "$log"
  fi
}

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
program_data_hex="${2:-}"
if [[ -n "$program_data_hex" && ! -f "$program_data_hex" ]]; then
  printf 'missing top-level program data hex input: %s\n' "$program_data_hex" >&2
  exit 1
fi
if [[ "$program_input" == *.c ]]; then
  if [[ -n "$program_data_hex" ]]; then
    printf '%s\n' "explicit data hex is only supported for raw .hex top-level program inputs" >&2
    exit 1
  fi
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
  if [[ -n "$program_data_hex" ]]; then
    printf '%s\n' "explicit data hex is only supported for raw .hex top-level program inputs" >&2
    exit 1
  fi
  program_hex="$(mktemp "${TMPDIR:-/tmp}/lnp64_top_program_from_asm.XXXXXX.hex")"
  program_data_hex="$(mktemp "${TMPDIR:-/tmp}/lnp64_top_program_data_from_asm.XXXXXX.hex")"
  tmp_files+=("$program_hex" "$program_data_hex")
  if [[ -n "${LNP64_BIN:-}" ]]; then
    "$LNP64_BIN" asm-flat-exec "$program_input" -o "$program_hex" --data-hex "$program_data_hex"
  else
    cargo run --quiet -- asm-flat-exec "$program_input" -o "$program_hex" --data-hex "$program_data_hex"
  fi
fi
if [[ "$program_input" == *.dump ]]; then
  if [[ -n "$program_data_hex" ]]; then
    printf '%s\n' "explicit data hex is only supported for raw .hex top-level program inputs" >&2
    exit 1
  fi
  program_hex="$(mktemp "${TMPDIR:-/tmp}/lnp64_top_program_from_llvm_dump.XXXXXX.hex")"
  tmp_files+=("$program_hex")
  python3 scripts/llvm_objdump_to_flat_hex.py "$program_input" -o "$program_hex"
fi

sim_log="$(mktemp "${TMPDIR:-/tmp}/lnp64_rtl_top_program_sim.XXXXXX.log")"
emulator_log="$(mktemp "${TMPDIR:-/tmp}/lnp64_emulator_top_program.XXXXXX.log")"
tmp_files+=("$sim_log" "$emulator_log")

rtl_binary="$build_dir/Vlnp64_top_program_tb"
if [[ "${LNP64_RTL_TOP_PROGRAM_SKIP_BUILD:-0}" == "1" ||
      "${LNP64_RTL_SKIP_BUILD:-0}" == "1" ]]; then
  if [[ ! -x "$rtl_binary" ]]; then
    printf 'missing reusable top-level RTL binary: %s\n' "$rtl_binary" >&2
    printf '%s\n' "unset LNP64_RTL_TOP_PROGRAM_SKIP_BUILD/LNP64_RTL_SKIP_BUILD or run one build first" >&2
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
run_top_program_logged "$sim_log" "$rtl_binary" "${rtl_plusargs[@]}"

if ! grep -q "LNP64-RTL-TOP-PROGRAM PASS" "$sim_log"; then
  printf '%s\n' "missing RTL top-level pass marker" >&2
  if [[ "$top_program_quiet" == "1" ]]; then
    cat "$sim_log" >&2
  fi
  exit 1
fi

if [[ -n "${LNP64_BIN:-}" ]]; then
  emulator_cmd=("$LNP64_BIN" run-flat-exec "$program_hex")
else
  emulator_cmd=(cargo run --quiet -- run-flat-exec "$program_hex")
fi
if [[ -n "$program_data_hex" && -s "$program_data_hex" ]]; then
  emulator_cmd+=(--data-hex "$program_data_hex")
fi
run_top_program_logged "$emulator_log" "${emulator_cmd[@]}"

python3 - "$sim_log" "$emulator_log" <<'PY'
import json
import re
import sys
from pathlib import Path


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


def load_m1_commit_schema() -> tuple[tuple[str, ...], tuple[int, ...]]:
    schema_path = Path("rtl/schema/lnp64_shared_schema.json")
    with schema_path.open(encoding="utf-8") as handle:
        schema = json.load(handle)
    entries = schema["records"]["lnp64_m1_cap_commit_t"]
    fields = []
    widths = []
    for entry in entries:
        field, width = entry.split(":", maxsplit=1)
        fields.append(field)
        widths.append(int(width))
    return tuple(fields), tuple(widths)


def parse_int_literal(value: str) -> int:
    value = value.strip()
    if "'h" in value:
        return int(value.split("'h", maxsplit=1)[1].replace("_", ""), 16)
    if "'d" in value:
        return int(value.split("'d", maxsplit=1)[1].replace("_", ""), 10)
    if value.lower().startswith("0x"):
        return int(value, 16)
    return int(value, 10)


def load_m1_commit_op_values() -> dict[str, int]:
    schema_path = Path("rtl/schema/lnp64_shared_schema.json")
    with schema_path.open(encoding="utf-8") as handle:
        schema = json.load(handle)
    enum_entries = schema["enums"]["lnp64_m1_commit_op_e"]
    values = {}
    for entry in enum_entries:
        name, value = entry.split("=", maxsplit=1)
        values[name] = parse_int_literal(value)
    return {
        "CapDup": values["LNP64_M1_COMMIT_CAP_DUP"],
        "CapSend": values["LNP64_M1_COMMIT_CAP_SEND"],
        "CapRecv": values["LNP64_M1_COMMIT_CAP_RECV"],
        "CapRevoke": values["LNP64_M1_COMMIT_CAP_REVOKE"],
    }


def load_flat_exec_m1_opcode_map() -> dict[int, int]:
    """Return flat-exec instruction opcodes, not architectural enum ids.

    The current top-level executable path consumes 8-bit flat-exec instruction
    words from src/main.rs and src/emulator.rs. The architectural opcode enum in
    rtl/include/lnp64_pkg.sv is wider and has already moved some capability
    operation ids, so the top-level commit check must follow the executable
    encoding until the loader/decode path is unified.
    """
    commit_ops = load_m1_commit_op_values()
    encoder = Path("src/main.rs").read_text(encoding="utf-8")
    decoder = Path("src/emulator.rs").read_text(encoding="utf-8")
    encoder_ops = {}
    decoder_ops = {}
    for instr in ("CapDup", "CapSend", "CapRecv", "CapRevoke"):
        encoder_match = re.search(
            rf"Instr::{instr}\([^)]*\)\s*=>\s*Ok\(vec!\[enc_rrr\((0x[0-9a-fA-F]+)",
            encoder,
        )
        if encoder_match is None:
            raise SystemExit(f"missing flat-exec encoder opcode for Instr::{instr}")
        decoder_match = re.search(
            rf"(0x[0-9a-fA-F]+)\s*=>\s*Instr::{instr}\(",
            decoder,
        )
        if decoder_match is None:
            raise SystemExit(f"missing flat-exec emulator decode opcode for Instr::{instr}")
        encoder_opcode = parse_int_literal(encoder_match.group(1))
        decoder_opcode = parse_int_literal(decoder_match.group(1))
        if encoder_opcode != decoder_opcode:
            raise SystemExit(
                f"flat-exec opcode drift for Instr::{instr}: "
                f"encoder=0x{encoder_opcode:x} decoder=0x{decoder_opcode:x}"
            )
        encoder_ops[encoder_opcode] = commit_ops[instr]
        decoder_ops[instr] = decoder_opcode
    if len(encoder_ops) != len(decoder_ops):
        raise SystemExit(f"duplicate flat-exec M1 opcodes detected: {decoder_ops}")
    return encoder_ops


def decode_packed_bits(bits: str, fields: tuple[str, ...], widths: tuple[int, ...]) -> dict[str, int]:
    total_width = sum(widths)
    try:
        raw = int(bits, 16)
    except ValueError as exc:
        raise SystemExit(f"invalid top-level M1 packed commit bits {bits!r}") from exc
    if raw >= (1 << total_width):
        raise SystemExit(
            f"top-level M1 packed commit bits exceed schema width {total_width}: 0x{bits}"
        )
    decoded = {}
    shift = total_width
    for field, width in zip(fields, widths, strict=True):
        shift -= width
        decoded[field] = (raw >> shift) & ((1 << width) - 1)
    if shift != 0:
        raise SystemExit("internal top-level M1 packed commit decoder did not consume all bits")
    return decoded


rtl = load_record(sys.argv[1], "RTL_FINAL ")
emulator = load_record(sys.argv[2], "EMULATOR_FINAL ")
if rtl["exit_reg"] != emulator["exit"]:
    raise SystemExit(f"exit mismatch: rtl={rtl['exit_reg']} emulator={emulator['exit']}")
for field in ("r3", "r4", "r5", "env_page", "mem0", "mem_checksum", "errno"):
    if rtl[field] != emulator[field]:
        raise SystemExit(f"{field} mismatch: rtl={rtl[field]} emulator={emulator[field]}")

rtl_retire = load_records(sys.argv[1], "RTL_RETIRE ")
emulator_retire = load_record(sys.argv[2], "EMULATOR_RETIRE ")
retire_required_fields = (
    "pc",
    "opcode",
    "tile_id",
    "pid",
    "tid",
    "domain_id",
    "domain_gen",
    "action",
    "operand_rd",
    "operand_rs1",
    "operand_rs2",
    "operand_rs3",
    "operand_imm",
    "result_valid",
    "result_reg",
    "result_value",
    "errno",
    "status",
    "event_id",
    "fault_id",
)
for label, records in (("rtl", rtl_retire), ("emulator", emulator_retire)):
    for idx, record in enumerate(records):
        missing = [field for field in retire_required_fields if field not in record]
        if missing:
            raise SystemExit(
                f"{label} retire record {idx} missing required field(s): {missing}"
            )
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

rtl_m1_top_commits = []
rtl_m1_top_commit_bits = []
with open(sys.argv[1], encoding="utf-8") as handle:
    for line in handle:
        if line.startswith("RTL_M1_TOP_COMMIT "):
            rtl_m1_top_commits.append(json.loads(line[len("RTL_M1_TOP_COMMIT "):]))
        elif line.startswith("RTL_M1_TOP_COMMIT_BITS "):
            rtl_m1_top_commit_bits.append(json.loads(line[len("RTL_M1_TOP_COMMIT_BITS "):]))
opcode_to_m1_op = load_flat_exec_m1_opcode_map()
cap_retire = [record for record in rtl_retire if record["opcode"] in opcode_to_m1_op]
if len(rtl_m1_top_commits) != len(cap_retire):
    raise SystemExit(
        "top-level M1 commit trace count mismatch: "
        f"cap_retire={len(cap_retire)} commits={len(rtl_m1_top_commits)}"
    )
if len(rtl_m1_top_commit_bits) != len(rtl_m1_top_commits):
    raise SystemExit(
        "top-level M1 packed commit trace count mismatch: "
        f"commits={len(rtl_m1_top_commits)} packed={len(rtl_m1_top_commit_bits)}"
    )
commit_required_fields = (
    "record",
    "op",
    "object_id",
    "object_gen",
    "fdr_gen",
    "domain_id",
    "domain_gen",
    "rights_mask",
    "lineage_epoch",
    "sealed",
    "status",
    "pc",
    "tile_id",
)
m1_schema_fields, m1_schema_widths = load_m1_commit_schema()
expected_m1_commit_width = sum(m1_schema_widths)
for idx, (commit, retire) in enumerate(zip(rtl_m1_top_commits, cap_retire)):
    missing = [field for field in commit_required_fields if field not in commit]
    if missing:
        raise SystemExit(f"top-level M1 commit {idx} missing required field(s): {missing}")
    if commit["record"] != "m1_cap_commit":
        raise SystemExit(f"top-level M1 commit {idx} has unexpected record {commit['record']!r}")
    if commit["op"] != opcode_to_m1_op[retire["opcode"]]:
        raise SystemExit(
            f"top-level M1 commit {idx} op mismatch: commit={commit['op']} "
            f"retire_opcode={retire['opcode']}"
        )
    if commit["pc"] != retire["pc"] or commit["tile_id"] != retire["tile_id"]:
        raise SystemExit(
            f"top-level M1 commit {idx} is not tied to retired instruction: "
            f"commit_pc_tile={(commit['pc'], commit['tile_id'])} "
            f"retire_pc_tile={(retire['pc'], retire['tile_id'])}"
        )
    if commit["status"] != retire["errno"]:
        raise SystemExit(
            f"top-level M1 commit {idx} status mismatch: "
            f"commit={commit['status']} retire_errno={retire['errno']}"
        )
for idx, (commit, bits_record) in enumerate(zip(rtl_m1_top_commits, rtl_m1_top_commit_bits)):
    if bits_record.get("record") != "m1_cap_commit_bits":
        raise SystemExit(
            f"top-level M1 packed commit {idx} has unexpected record "
            f"{bits_record.get('record')!r}"
        )
    if bits_record.get("width") != expected_m1_commit_width:
        raise SystemExit(
            f"top-level M1 packed commit {idx} has unexpected width "
            f"{bits_record.get('width')!r}; expected schema width {expected_m1_commit_width}"
        )
    if "bits" not in bits_record:
        raise SystemExit(f"top-level M1 packed commit {idx} is missing bits")
    if bits_record.get("pc") != commit["pc"] or bits_record.get("tile_id") != commit["tile_id"]:
        raise SystemExit(
            f"top-level M1 packed commit {idx} is not tied to JSON commit: "
            f"bits_pc_tile={(bits_record.get('pc'), bits_record.get('tile_id'))} "
            f"commit_pc_tile={(commit['pc'], commit['tile_id'])}"
        )
    decoded = decode_packed_bits(bits_record["bits"], m1_schema_fields, m1_schema_widths)
    for field in m1_schema_fields:
        if decoded[field] != commit[field]:
            raise SystemExit(
                f"top-level M1 packed commit {idx} field {field} drifted from JSON commit: "
                f"packed={decoded[field]} json={commit[field]}"
            )
PY

printf '%s\n' "rtl top-level program smoke ok"

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
rtl_lock_build_dir "$build_dir"
if [[ "${LNP64_RTL_TOP_PROGRAM_SKIP_BUILD:-0}" == "1" ||
      "${LNP64_RTL_SKIP_BUILD:-0}" == "1" ]]; then
  if [[ ! -x "$rtl_binary" ]]; then
    printf 'missing reusable top-level RTL binary: %s\n' "$rtl_binary" >&2
    printf '%s\n' "unset LNP64_RTL_TOP_PROGRAM_SKIP_BUILD/LNP64_RTL_SKIP_BUILD or run one build first" >&2
    exit 1
  fi
else
  rtl_prepare_build_dir "$build_dir"
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
if [[ "${LNP64_RTL_TOP_PROGRAM_CROSS_TILE_WAKE:-0}" == "1" ]]; then
  rtl_plusargs+=("+lnp64_inject_cross_tile_wake")
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
import os
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
    return load_schema_record("lnp64_m1_cap_commit_t")


def load_m1_state_schema() -> tuple[tuple[str, ...], tuple[int, ...]]:
    return load_schema_record("lnp64_m1_state_projection_t")


def load_schema_record(record_name: str) -> tuple[tuple[str, ...], tuple[int, ...]]:
    schema_path = Path("rtl/schema/lnp64_shared_schema.json")
    with schema_path.open(encoding="utf-8") as handle:
        schema = json.load(handle)
    entries = schema["records"][record_name]
    fields = []
    widths = []
    for entry in entries:
        field, width = entry.split(":", maxsplit=1)
        fields.append(field)
        widths.append(int(width))
    return tuple(fields), tuple(widths)


def load_schema_enum_values(enum_name: str) -> dict[str, int]:
    schema_path = Path("rtl/schema/lnp64_shared_schema.json")
    with schema_path.open(encoding="utf-8") as handle:
        schema = json.load(handle)
    enum_entries = schema["enums"][enum_name]
    values = {}
    for entry in enum_entries:
        name, value = entry.split("=", maxsplit=1)
        values[name] = parse_int_literal(value)
    return values


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
    values = load_schema_enum_values("lnp64_m1_commit_op_e")
    return {
        "CapDup": values["LNP64_M1_COMMIT_CAP_DUP"],
        "CapSend": values["LNP64_M1_COMMIT_CAP_SEND"],
        "CapRecv": values["LNP64_M1_COMMIT_CAP_RECV"],
        "CapRevoke": values["LNP64_M1_COMMIT_CAP_REVOKE"],
    }


def load_arch_m1_opcode_map() -> dict[int, int]:
    opcodes = load_schema_enum_values("lnp64_opcode_e")
    commit_ops = load_m1_commit_op_values()
    return {
        opcodes["LNP64_OP_CAP_DUP"]: commit_ops["CapDup"],
        opcodes["LNP64_OP_CAP_SEND"]: commit_ops["CapSend"],
        opcodes["LNP64_OP_CAP_RECV"]: commit_ops["CapRecv"],
        opcodes["LNP64_OP_CAP_REVOKE"]: commit_ops["CapRevoke"],
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


def load_flat_to_arch_opcode_map() -> dict[int, int]:
    opcodes = load_schema_enum_values("lnp64_opcode_e")
    decode_source = Path("rtl/core/lnp64_decode.sv").read_text(encoding="utf-8")
    pairs = re.findall(
        r"8'h([0-9a-fA-F]+):\s*begin\s*dec\.opcode\s*=\s*(LNP64_OP_[A-Z0-9_]+)\s*;",
        decode_source,
        flags=re.MULTILINE,
    )
    flat_to_arch = {}
    for raw_hex, arch_name in pairs:
        if arch_name not in opcodes:
            raise SystemExit(f"RTL decode maps flat opcode 0x{raw_hex} to unknown {arch_name}")
        raw_opcode = int(raw_hex, 16)
        if raw_opcode in flat_to_arch:
            raise SystemExit(f"duplicate RTL flat opcode decode entry: 0x{raw_opcode:02x}")
        flat_to_arch[raw_opcode] = opcodes[arch_name]
    if not flat_to_arch:
        raise SystemExit("could not parse any flat-to-architectural opcode mappings")
    return flat_to_arch


def load_rust_flat_to_arch_opcode_map() -> dict[int, int]:
    opcodes = load_schema_enum_values("lnp64_opcode_e")
    emulator_source = Path("src/emulator.rs").read_text(encoding="utf-8")
    direct_instr_to_arch = {
        "Nop": "LNP64_OP_NOP",
        "Mov": "LNP64_OP_MOV",
        "Add": "LNP64_OP_ADD",
        "Addi": "LNP64_OP_ADDI",
        "Sub": "LNP64_OP_SUB",
        "Mul": "LNP64_OP_MUL",
        "Mulh": "LNP64_OP_MULH",
        "Mulhu": "LNP64_OP_MULHU",
        "Mulhsu": "LNP64_OP_MULHSU",
        "Div": "LNP64_OP_DIV",
        "Udiv": "LNP64_OP_UDIV",
        "Srem": "LNP64_OP_SREM",
        "Urem": "LNP64_OP_UREM",
        "And": "LNP64_OP_AND",
        "Andi": "LNP64_OP_ANDI",
        "Or": "LNP64_OP_OR",
        "Ori": "LNP64_OP_ORI",
        "Xor": "LNP64_OP_XOR",
        "Xori": "LNP64_OP_XORI",
        "Not": "LNP64_OP_NOT",
        "Lsl": "LNP64_OP_LSL",
        "Lsli": "LNP64_OP_LSLI",
        "Lsr": "LNP64_OP_LSR",
        "Lsri": "LNP64_OP_LSRI",
        "Asr": "LNP64_OP_ASR",
        "Asri": "LNP64_OP_ASRI",
        "SextB": "LNP64_OP_SEXT_B",
        "SextH": "LNP64_OP_SEXT_H",
        "SextW": "LNP64_OP_SEXT_W",
        "ZextB": "LNP64_OP_ZEXT_B",
        "ZextH": "LNP64_OP_ZEXT_H",
        "ZextW": "LNP64_OP_ZEXT_W",
        "Clz": "LNP64_OP_CLZ",
        "Ctz": "LNP64_OP_CTZ",
        "Popcnt": "LNP64_OP_POPCNT",
        "Rol": "LNP64_OP_ROL",
        "Ror": "LNP64_OP_ROR",
        "Bswap16": "LNP64_OP_BSWAP16",
        "Bswap32": "LNP64_OP_BSWAP32",
        "Bswap64": "LNP64_OP_BSWAP64",
        "Cmp": "LNP64_OP_CMP",
        "Cmpu": "LNP64_OP_CMPU",
        "Jmp": "LNP64_OP_JMP",
        "Call": "LNP64_OP_CALL",
        "CallReg": "LNP64_OP_CALL_REG",
        "LrGet": "LNP64_OP_LR_GET",
        "LrSet": "LNP64_OP_LR_SET",
        "Ret": "LNP64_OP_RET",
        "Yield": "LNP64_OP_YIELD",
        "Sleep": "LNP64_OP_SLEEP",
        "Pull": "LNP64_OP_PULL",
        "Push": "LNP64_OP_PUSH",
        "PullDyn": "LNP64_OP_PULL",
        "PushDyn": "LNP64_OP_PUSH",
        "Await": "LNP64_OP_AWAIT",
        "AwaitDyn": "LNP64_OP_AWAIT",
        "CallCap": "LNP64_OP_GATE_CALL",
        "CallCapDyn": "LNP64_OP_GATE_CALL",
        "RetCap": "LNP64_OP_GATE_RETURN",
        "ErrnoGet": "LNP64_OP_GET_ERRNO",
        "ErrnoSet": "LNP64_OP_SET_ERRNO",
        "Exit": "LNP64_OP_EXIT",
        "Fence": "LNP64_OP_FENCE",
        "Isync": "LNP64_OP_ISYNC",
        "Alloc": "LNP64_OP_ALLOC",
        "AllocSize": "LNP64_OP_ALLOC_SIZE",
        "Free": "LNP64_OP_FREE",
        "AllocEx": "LNP64_OP_ALLOC_EX",
        "ObjectCtl": "LNP64_OP_OBJECT_CTL",
        "CapDup": "LNP64_OP_CAP_DUP",
        "CapSend": "LNP64_OP_CAP_SEND",
        "CapRecv": "LNP64_OP_CAP_RECV",
        "CapRevoke": "LNP64_OP_CAP_REVOKE",
        "DmaCtl": "LNP64_OP_DMA_CTL",
        "EnvGet": "LNP64_OP_ENV_GET",
        "ReadFd": "LNP64_OP_READ_FD",
        "WriteFd": "LNP64_OP_WRITE_FD",
        "AmoSwap": "LNP64_OP_AMO_SWAP",
        "AmoAdd": "LNP64_OP_AMO_ADD",
        "AmoAnd": "LNP64_OP_AMO_AND",
        "AmoOr": "LNP64_OP_AMO_OR",
        "AmoXor": "LNP64_OP_AMO_XOR",
        "LockCmpxchg": "LNP64_OP_LOCK_CMPXCHG",
    }
    flat_to_arch = {
        0x01: opcodes["LNP64_OP_LI32"],
        0x03: opcodes["LNP64_OP_LA_LITERAL"],
        0x04: opcodes["LNP64_OP_LI32_LITERAL"],
        0x20: opcodes["LNP64_OP_JMP"],
        0x21: opcodes["LNP64_OP_BRANCH_EQ"],
        0x22: opcodes["LNP64_OP_BRANCH_NE"],
        0x23: opcodes["LNP64_OP_BRANCH_LT"],
        0x24: opcodes["LNP64_OP_BRANCH_GT"],
        0x25: opcodes["LNP64_OP_BRANCH_LE"],
        0x26: opcodes["LNP64_OP_BRANCH_GE"],
        0x30: opcodes["LNP64_OP_LD"],
        0x31: opcodes["LNP64_OP_LD_W"],
        0x32: opcodes["LNP64_OP_LD_B"],
        0x33: opcodes["LNP64_OP_ST"],
        0x34: opcodes["LNP64_OP_ST_W"],
        0x35: opcodes["LNP64_OP_ST_B"],
        0x36: opcodes["LNP64_OP_LD_H"],
        0x37: opcodes["LNP64_OP_ST_H"],
        0x3D: opcodes["LNP64_OP_CSET_EQ"],
        0x3E: opcodes["LNP64_OP_CSET_NE"],
        0x3F: opcodes["LNP64_OP_CSET_LT"],
        0x40: opcodes["LNP64_OP_CSET_GT"],
        0x41: opcodes["LNP64_OP_CSET_LE"],
        0x42: opcodes["LNP64_OP_CSET_GE"],
        0x43: opcodes["LNP64_OP_CSET_ULT"],
        0x44: opcodes["LNP64_OP_CSET_UGT"],
        0x45: opcodes["LNP64_OP_CSET_ULE"],
        0x46: opcodes["LNP64_OP_CSET_UGE"],
        0xBB: opcodes["LNP64_OP_CSEL_EQ"],
        0xBC: opcodes["LNP64_OP_CSEL_NE"],
        0xBD: opcodes["LNP64_OP_CSEL_LT"],
        0xBE: opcodes["LNP64_OP_CSEL_GT"],
        0xBF: opcodes["LNP64_OP_CSEL_LE"],
        0xC0: opcodes["LNP64_OP_CSEL_GE"],
        0xC1: opcodes["LNP64_OP_CSEL_ULT"],
        0xC2: opcodes["LNP64_OP_CSEL_UGT"],
        0xC3: opcodes["LNP64_OP_CSEL_ULE"],
        0xC4: opcodes["LNP64_OP_CSEL_UGE"],
        0xD0: opcodes["LNP64_OP_AUIPC_LITERAL"],
        0xFF: opcodes["LNP64_OP_UNSUPPORTED"],
    }
    for raw_hex, instr_name in re.findall(
        r"(0x[0-9a-fA-F]+)\s*=>\s*Instr::([A-Za-z0-9_]+)\b",
        emulator_source,
    ):
        raw_opcode = parse_int_literal(raw_hex)
        arch_name = direct_instr_to_arch.get(instr_name)
        if arch_name is not None:
            flat_to_arch[raw_opcode] = opcodes[arch_name]
    return flat_to_arch


def check_rtl_decode_matches_rust(rtl_flat_to_arch: dict[int, int], rust_flat_to_arch: dict[int, int]) -> None:
    shared_opcodes = sorted(set(rtl_flat_to_arch) & set(rust_flat_to_arch))
    mismatches = [
        (opcode, rtl_flat_to_arch[opcode], rust_flat_to_arch[opcode])
        for opcode in shared_opcodes
        if rtl_flat_to_arch[opcode] != rust_flat_to_arch[opcode]
    ]
    if mismatches:
        raise SystemExit(f"RTL/Rust flat-to-architectural opcode drift: {mismatches}")
    missing_in_rust = sorted(set(rtl_flat_to_arch) - set(rust_flat_to_arch))
    if missing_in_rust:
        raise SystemExit(f"RTL decode has flat opcodes missing from Rust committed exec map: {missing_in_rust}")


def check_cross_tile_wake_event(path: str) -> None:
    events = collect_json_records(path, "RTL_EVENT ")
    if len(events) != 1:
        raise SystemExit(f"cross-tile wake expected exactly one RTL_EVENT record, saw {len(events)}")
    event = events[0]
    statuses = load_schema_enum_values("lnp64_status_e")
    engines = load_schema_enum_values("lnp64_engine_e")
    expected = {
        "record": "event",
        "tile_id": 0,
        "source_tile_id": 1,
        "op_id": 0,
        "pid": 1,
        "tid": 1,
        "domain_id": 1,
        "domain_gen": 1,
        "event_mask": 1,
        "source": engines["LNP64_ENGINE_NONE"],
        "status": statuses["LNP64_STATUS_EVENT"],
        "wake_valid": 1,
        "cross_tile_wake": 1,
    }
    for field, expected_value in expected.items():
        if event.get(field) != expected_value:
            raise SystemExit(
                f"cross-tile wake event field {field} mismatch: "
                f"rtl={event.get(field)!r} expected={expected_value!r}"
            )
    if not isinstance(event.get("event_id"), int) or event["event_id"] <= 0:
        raise SystemExit(f"cross-tile wake event has invalid event_id: {event.get('event_id')!r}")


def add_expected_arch_opcodes(records: list[dict], flat_to_arch: dict[int, int]) -> None:
    for idx, record in enumerate(records):
        opcode = record.get("opcode")
        if not isinstance(opcode, int):
            raise SystemExit(f"retire record {idx} has invalid opcode {opcode!r}")
        record["arch_opcode"] = flat_to_arch.get(opcode, opcode)


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


def collect_json_records(path: str, prefix: str) -> list[dict]:
    records = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if not line.startswith(prefix):
                continue
            payload = line.removeprefix(prefix)
            try:
                records.append(json.loads(payload))
            except json.JSONDecodeError as exc:
                raise SystemExit(f"invalid {prefix.strip()} record {payload!r}: {exc}") from exc
    return records


def require_top_state_records(
    label: str,
    records: list[dict],
    bit_records: list[dict],
    commits: list[dict],
    state_fields: tuple[str, ...],
    state_widths: tuple[int, ...],
) -> None:
    if len(records) != len(commits):
        raise SystemExit(
            f"top-level M1 {label} state count mismatch: "
            f"commits={len(commits)} states={len(records)}"
        )
    if len(bit_records) != len(records):
        raise SystemExit(
            f"top-level M1 {label} packed state count mismatch: "
            f"states={len(records)} packed={len(bit_records)}"
        )
    expected_width = sum(state_widths)
    for idx, (record, bits_record, commit) in enumerate(zip(records, bit_records, commits)):
        missing = [
            field
            for field in ("record", *state_fields, "pc", "tile_id")
            if field not in record
        ]
        if missing:
            raise SystemExit(f"top-level M1 {label} state {idx} missing field(s): {missing}")
        if record["record"] != "m1_state_projection":
            raise SystemExit(
                f"top-level M1 {label} state {idx} has unexpected record {record['record']!r}"
            )
        if record["pc"] != commit["pc"] or record["tile_id"] != commit["tile_id"]:
            raise SystemExit(
                f"top-level M1 {label} state {idx} is not tied to commit: "
                f"state_pc_tile={(record['pc'], record['tile_id'])} "
                f"commit_pc_tile={(commit['pc'], commit['tile_id'])}"
            )
        if record["op"] != commit["op"] or record["status"] != commit["status"]:
            raise SystemExit(
                f"top-level M1 {label} state {idx} op/status drifted from commit: "
                f"state={(record['op'], record['status'])} commit={(commit['op'], commit['status'])}"
            )
        if bits_record.get("record") != "m1_state_projection_bits":
            raise SystemExit(
                f"top-level M1 {label} packed state {idx} has unexpected record "
                f"{bits_record.get('record')!r}"
            )
        if bits_record.get("width") != expected_width:
            raise SystemExit(
                f"top-level M1 {label} packed state {idx} width drifted from schema: "
                f"{bits_record.get('width')!r} != {expected_width}"
            )
        if bits_record.get("pc") != commit["pc"] or bits_record.get("tile_id") != commit["tile_id"]:
            raise SystemExit(
                f"top-level M1 {label} packed state {idx} is not tied to commit: "
                f"bits_pc_tile={(bits_record.get('pc'), bits_record.get('tile_id'))} "
                f"commit_pc_tile={(commit['pc'], commit['tile_id'])}"
            )
        bits = bits_record.get("bits")
        if not isinstance(bits, str):
            raise SystemExit(f"top-level M1 {label} packed state {idx} is missing bits")
        decoded = decode_packed_bits(bits, state_fields, state_widths)
        for field in state_fields:
            if decoded[field] != record[field]:
                raise SystemExit(
                    f"top-level M1 {label} packed state {idx} field {field} drifted: "
                    f"packed={decoded[field]} json={record[field]}"
                )


def rights_subset(child: int, parent: int) -> bool:
    return (child & ~parent) == 0


def check_top_m1_projection_matches_commit(
    prefix: str,
    commit: dict,
    state: dict,
    idx: int,
    transition: str,
) -> None:
    for state_field, commit_field in (
        (f"{prefix}_object_id", "object_id"),
        (f"{prefix}_generation", "fdr_gen"),
        (f"{prefix}_rights", "rights_mask"),
        (f"{prefix}_lineage_epoch", "lineage_epoch"),
        (f"{prefix}_sealed", "sealed"),
    ):
        if state[state_field] != commit[commit_field]:
            raise SystemExit(
                f"top-level M1 {transition} {idx} {prefix} projection {state_field} "
                f"does not match commit {commit_field}: "
                f"state={state[state_field]} commit={commit[commit_field]}"
            )


def check_top_m1_non_ok_transition(
    idx: int,
    commit: dict,
    pre_state: dict,
    post_state: dict,
    authority_projection_fields: tuple[str, ...],
) -> None:
    drift = [
        (field, pre_state[field], post_state[field])
        for field in authority_projection_fields
        if pre_state[field] != post_state[field]
    ]
    if drift:
        raise SystemExit(
            f"top-level M1 non-OK commit {idx} changed authority projection: {drift}"
        )
    status = commit["status"]
    if status == 116 and post_state["stale_rejected"] != 1:
        raise SystemExit(f"top-level M1 non-OK commit {idx} did not mark stale rejection")
    if status in (1, 9) and post_state["failed_no_authority"] != 1:
        raise SystemExit(f"top-level M1 non-OK commit {idx} did not mark failed authority")
    if status == 11 and post_state["full_was_explicit"] != 1:
        raise SystemExit(f"top-level M1 non-OK commit {idx} did not mark explicit full queue")


def check_top_m1_refinement_step(
    idx: int,
    commit: dict,
    pre_state: dict,
    post_state: dict,
    commit_ops: dict[str, int],
    authority_projection_fields: tuple[str, ...],
) -> None:
    """Executable top-level mirror of the current M1 RTL-to-Lean step shape."""
    if commit["status"] != 0:
        check_top_m1_non_ok_transition(
            idx,
            commit,
            pre_state,
            post_state,
            authority_projection_fields,
        )
        return

    op = commit["op"]
    if op == commit_ops["CapDup"]:
        if pre_state["root_rights"] & 0x40 == 0:
            raise SystemExit(f"top-level M1 capDup {idx} accepted without DUP right")
        if not rights_subset(commit["rights_mask"], pre_state["root_rights"]):
            raise SystemExit(
                f"top-level M1 capDup {idx} amplified rights: "
                f"commit={commit['rights_mask']} pre_root={pre_state['root_rights']}"
            )
        if commit["object_id"] != pre_state["root_object_id"]:
            raise SystemExit(f"top-level M1 capDup {idx} changed object lineage")
        check_top_m1_projection_matches_commit("consumer", commit, post_state, idx, "capDup")
        return

    if op == commit_ops["CapSend"]:
        if pre_state["sent_valid"] != 0:
            raise SystemExit(f"top-level M1 capSend {idx} accepted with occupied transfer slot")
        if pre_state["consumer_rights"] & 0x100 == 0:
            raise SystemExit(f"top-level M1 capSend {idx} accepted without TRANSFER right")
        if not rights_subset(commit["rights_mask"], pre_state["consumer_rights"]):
            raise SystemExit(
                f"top-level M1 capSend {idx} amplified rights: "
                f"commit={commit['rights_mask']} pre_consumer={pre_state['consumer_rights']}"
            )
        if post_state["sent_valid"] != 1:
            raise SystemExit(f"top-level M1 capSend {idx} did not publish a sent cap")
        check_top_m1_projection_matches_commit("sent", commit, post_state, idx, "capSend")
        return

    if op == commit_ops["CapRecv"]:
        if pre_state["sent_valid"] != 1:
            raise SystemExit(f"top-level M1 capRecv {idx} accepted without a sent cap")
        if not rights_subset(commit["rights_mask"], pre_state["sent_rights"]):
            raise SystemExit(
                f"top-level M1 capRecv {idx} amplified rights: "
                f"commit={commit['rights_mask']} pre_sent={pre_state['sent_rights']}"
            )
        if post_state["sent_valid"] != 0:
            raise SystemExit(f"top-level M1 capRecv {idx} left a sent cap queued")
        check_top_m1_projection_matches_commit("consumer", commit, post_state, idx, "capRecv")
        return

    if op == commit_ops["CapRevoke"]:
        if not rights_subset(commit["rights_mask"], pre_state["root_rights"]):
            raise SystemExit(
                f"top-level M1 capRevoke {idx} commit rights exceed pre root rights: "
                f"commit={commit['rights_mask']} pre_root={pre_state['root_rights']}"
            )
        if post_state["has_revoked_generation"] == 1:
            if post_state["revoked_generation"] != commit["fdr_gen"]:
                raise SystemExit(
                    f"top-level M1 capRevoke {idx} revoked generation does not match commit: "
                    f"post={post_state['revoked_generation']} commit={commit['fdr_gen']}"
                )
            if post_state["root_generation"] != commit["fdr_gen"]:
                raise SystemExit(
                    f"top-level M1 capRevoke {idx} root generation does not match commit: "
                    f"post={post_state['root_generation']} commit={commit['fdr_gen']}"
                )
        return

    raise SystemExit(f"top-level M1 accepted unsupported transition op {op}")


rtl = load_record(sys.argv[1], "RTL_FINAL ")
emulator = load_record(sys.argv[2], "EMULATOR_FINAL ")
if rtl["exit_reg"] != emulator["exit"]:
    raise SystemExit(f"exit mismatch: rtl={rtl['exit_reg']} emulator={emulator['exit']}")
if not isinstance(rtl.get("regs"), list) or len(rtl["regs"]) != 32:
    raise SystemExit(f"RTL final register file is missing or malformed: {rtl.get('regs')!r}")
if not isinstance(emulator.get("regs"), list) or len(emulator["regs"]) != 32:
    raise SystemExit(
        f"emulator final register file is missing or malformed: {emulator.get('regs')!r}"
    )
if rtl["regs"] != emulator["regs"]:
    diffs = [
        (idx, rtl["regs"][idx], emulator["regs"][idx])
        for idx in range(32)
        if rtl["regs"][idx] != emulator["regs"][idx]
    ]
    raise SystemExit(f"final register file mismatch: {diffs}")
for field in ("r3", "r4", "r5", "env_page", "mem0", "mem_checksum", "errno"):
    if rtl[field] != emulator[field]:
        raise SystemExit(f"{field} mismatch: rtl={rtl[field]} emulator={emulator[field]}")

rtl_retire = load_records(sys.argv[1], "RTL_RETIRE ")
emulator_retire = load_record(sys.argv[2], "EMULATOR_RETIRE ")
flat_to_arch_opcode = load_flat_to_arch_opcode_map()
rust_flat_to_arch_opcode = load_rust_flat_to_arch_opcode_map()
check_rtl_decode_matches_rust(flat_to_arch_opcode, rust_flat_to_arch_opcode)
add_expected_arch_opcodes(emulator_retire, flat_to_arch_opcode)
retire_required_fields = (
    "pc",
    "opcode",
    "arch_opcode",
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
        if record["arch_opcode"] != flat_to_arch_opcode.get(record["opcode"], record["opcode"]):
            raise SystemExit(
                f"{label} retire record {idx} arch opcode mismatch: "
                f"flat={record['opcode']} arch={record['arch_opcode']} "
                f"expected={flat_to_arch_opcode.get(record['opcode'], record['opcode'])}"
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

if os.environ.get("LNP64_RTL_TOP_PROGRAM_CROSS_TILE_WAKE") == "1":
    check_cross_tile_wake_event(sys.argv[1])

rtl_m1_top_commits = []
rtl_m1_top_commit_bits = []
rtl_m1_top_pre_states = []
rtl_m1_top_pre_state_bits = []
rtl_m1_top_states = []
rtl_m1_top_state_bits = []
with open(sys.argv[1], encoding="utf-8") as handle:
    for line in handle:
        if line.startswith("RTL_M1_TOP_COMMIT "):
            rtl_m1_top_commits.append(json.loads(line[len("RTL_M1_TOP_COMMIT "):]))
        elif line.startswith("RTL_M1_TOP_COMMIT_BITS "):
            rtl_m1_top_commit_bits.append(json.loads(line[len("RTL_M1_TOP_COMMIT_BITS "):]))
        elif line.startswith("RTL_M1_TOP_PRE_STATE "):
            rtl_m1_top_pre_states.append(json.loads(line[len("RTL_M1_TOP_PRE_STATE "):]))
        elif line.startswith("RTL_M1_TOP_PRE_STATE_BITS "):
            rtl_m1_top_pre_state_bits.append(json.loads(line[len("RTL_M1_TOP_PRE_STATE_BITS "):]))
        elif line.startswith("RTL_M1_TOP_STATE "):
            rtl_m1_top_states.append(json.loads(line[len("RTL_M1_TOP_STATE "):]))
        elif line.startswith("RTL_M1_TOP_STATE_BITS "):
            rtl_m1_top_state_bits.append(json.loads(line[len("RTL_M1_TOP_STATE_BITS "):]))
load_flat_exec_m1_opcode_map()
arch_opcode_to_m1_op = load_arch_m1_opcode_map()
commit_ops = load_m1_commit_op_values()
cap_retire = [record for record in rtl_retire if record["arch_opcode"] in arch_opcode_to_m1_op]
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
    if commit["op"] != arch_opcode_to_m1_op[retire["arch_opcode"]]:
        raise SystemExit(
            f"top-level M1 commit {idx} op mismatch: commit={commit['op']} "
            f"retire_opcode={retire['opcode']} retire_arch_opcode={retire['arch_opcode']}"
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
m1_state_fields, m1_state_widths = load_m1_state_schema()
require_top_state_records(
    "pre",
    rtl_m1_top_pre_states,
    rtl_m1_top_pre_state_bits,
    rtl_m1_top_commits,
    m1_state_fields,
    m1_state_widths,
)
require_top_state_records(
    "post",
    rtl_m1_top_states,
    rtl_m1_top_state_bits,
    rtl_m1_top_commits,
    m1_state_fields,
    m1_state_widths,
)

authority_projection_fields = tuple(
    field
    for field in m1_state_fields
    if field
    not in {
        "op",
        "status",
        "stale_rejected",
        "revoked_rejected",
        "failed_no_authority",
        "full_was_explicit",
    }
)
for idx, (commit, pre_state, post_state) in enumerate(
    zip(rtl_m1_top_commits, rtl_m1_top_pre_states, rtl_m1_top_states)
):
    check_top_m1_refinement_step(
        idx,
        commit,
        pre_state,
        post_state,
        commit_ops,
        authority_projection_fields,
    )
PY

printf '%s\n' "rtl top-level program smoke ok"

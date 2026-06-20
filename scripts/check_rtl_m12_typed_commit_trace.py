#!/usr/bin/env python3
"""Check the M12 RTL typed storage-barrier commit trace.

Follows the M1/M7/M10/M11 typed-trace pattern for the SG-MEM storage device
slice. The lnp64_m12_storage_barrier engine emits a schema-owned packed commit
(lnp64_m12_storage_commit_t) and state projection (lnp64_m12_state_projection_t)
per storage transition: boot-image read, authorized block write, storage
barrier (issued and quiescent), stale-object rejection, cross-domain rejection,
terminal media fault, and the raw-authority retirement. This checker decodes
the packed bit vectors against the shared schema, verifies the seed-0 commit op
sequence, and checks the authority-relevant per-op invariants -- object/domain
binding, never-exposed raw device authority, and the rejection invariants
holding before the terminal media fault.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "rtl/schema/lnp64_shared_schema.json"
DEFAULT_M12_TRACE_LOG = Path("/tmp/lnp64_rtl_m12_typed_commit.log")

COMMIT_NAME = "m12_storage_commit"
STATE_NAME = "m12_state_projection"
COMMIT_BITS_NAME = "m12_storage_commit_bits"
STATE_BITS_NAME = "m12_state_projection_bits"
COMMIT_RECORD = "lnp64_m12_storage_commit_t"
STATE_RECORD = "lnp64_m12_state_projection_t"
OP_ENUM = "lnp64_m12_storage_op_e"

WITNESS_SCHEMA = "lnp64_m12_storage_refinement_witness_v1"

ERR_OK = 0
ERR_EPERM = 1
ERR_EIO = 5
ERR_EREVOKED = 122


def fail(message: str) -> None:
    raise SystemExit(f"M12 typed commit check failed: {message}")


def parse_sv_int(value: str) -> int:
    value = value.strip()
    if "'h" in value:
        return int(value.split("'h", maxsplit=1)[1].replace("_", ""), 16)
    if "'d" in value:
        return int(value.split("'d", maxsplit=1)[1].replace("_", ""), 10)
    return int(value, 0)


def parse_schema_field(entry: str) -> tuple[str, int]:
    name, width = entry.split(":", maxsplit=1)
    return name, int(width)


def load_schema() -> tuple[tuple[str, ...], tuple[int, ...], tuple[str, ...], tuple[int, ...], dict[str, int]]:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    commit = [parse_schema_field(e) for e in schema["records"][COMMIT_RECORD]]
    state = [parse_schema_field(e) for e in schema["records"][STATE_RECORD]]
    ops = {}
    for entry in schema["enums"][OP_ENUM]:
        name, value = entry.split("=", maxsplit=1)
        ops[name] = parse_sv_int(value)
    commit_fields = tuple(n for n, _ in commit)
    commit_widths = tuple(w for _, w in commit)
    state_fields = tuple(n for n, _ in state)
    state_widths = tuple(w for _, w in state)
    return commit_fields, commit_widths, state_fields, state_widths, ops


def run_m12_gate() -> str:
    log_path = Path(os.environ.get("LNP64_M12_TYPED_COMMIT_LOG", DEFAULT_M12_TRACE_LOG))
    if os.environ.get("LNP64_M12_TYPED_COMMIT_USE_EXISTING") == "1":
        try:
            return log_path.read_text(encoding="utf-8")
        except OSError as exc:
            fail(f"missing existing M12 typed commit log {log_path}: {exc}")
    proc = subprocess.run(
        ["bash", "scripts/run_rtl_m12.sh"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if proc.returncode != 0:
        print(proc.stdout, end="")
        fail(f"scripts/run_rtl_m12.sh exited with {proc.returncode}")
    try:
        log_path.write_text(proc.stdout, encoding="utf-8")
    except OSError:
        pass
    return proc.stdout


def require_int(record: dict, key: str) -> int:
    value = record.get(key)
    if not isinstance(value, int):
        fail(f"record {record.get('record')} has non-integer {key}: {value!r}")
    return value


def parse_json_records(output: str, prefix: str, record_name: str, fields: tuple[str, ...]) -> list[dict]:
    parsed = []
    for line in output.splitlines():
        if not line.startswith(prefix):
            continue
        payload = line[len(prefix):]
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid JSON record {payload!r}: {exc}")
        if record.get("record") != record_name:
            fail(f"unexpected record type {record.get('record')!r}")
        actual = tuple(k for k in record if k != "record")
        if actual != fields:
            fail(f"{record_name} fields drifted: {actual!r} != {fields!r}")
        for field in fields:
            require_int(record, field)
        parsed.append(record)
    if not parsed:
        fail(f"no {prefix.strip()} records emitted")
    return parsed


def parse_bit_records(output: str, prefix: str, record_name: str, expected_width: int) -> list[str]:
    parsed = []
    for line in output.splitlines():
        if not line.startswith(prefix):
            continue
        payload = line[len(prefix):]
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid packed bit record {payload!r}: {exc}")
        if record.get("record") != record_name:
            fail(f"unexpected packed bit record type {record.get('record')!r}")
        if record.get("width") != expected_width:
            fail(f"packed bit record {record_name} width drifted from schema: {record.get('width')!r} != {expected_width}")
        bits = record.get("bits")
        if not isinstance(bits, str) or not re.fullmatch(r"[0-9a-fA-F]+", bits):
            fail(f"packed bit record {record_name} has invalid bits {bits!r}")
        parsed.append(bits)
    if not parsed:
        fail(f"no {prefix.strip()} records emitted")
    return parsed


def decode_packed_bits(bits: str, fields: tuple[str, ...], widths: tuple[int, ...]) -> dict[str, int]:
    total = sum(widths)
    value = int(bits, 16)
    if value >= (1 << total):
        fail(f"packed bits exceed schema width {total}: 0x{bits}")
    decoded = {}
    shift = total
    for field, width in zip(fields, widths, strict=True):
        shift -= width
        decoded[field] = (value >> shift) & ((1 << width) - 1)
    return decoded


def check_bits(records: list[dict], bits: list[str], fields: tuple[str, ...], widths: tuple[int, ...], label: str) -> None:
    if len(records) != len(bits):
        fail(f"{label} packed bit count {len(bits)} != record count {len(records)}")
    for index, (record, bit_record) in enumerate(zip(records, bits, strict=True)):
        decoded = decode_packed_bits(bit_record, fields, widths)
        for field in fields:
            if decoded[field] != require_int(record, field):
                fail(f"{label} packed decode drift at {index} field {field}: {decoded[field]} != {record[field]}")


def expected_sequence(ops: dict[str, int]) -> list[int]:
    return [
        ops["LNP64_M12_COMMIT_BOOT_IMAGE"],
        ops["LNP64_M12_COMMIT_BLOCK_WRITE"],
        ops["LNP64_M12_COMMIT_BARRIER"],
        ops["LNP64_M12_COMMIT_STALE_OBJECT"],
        ops["LNP64_M12_COMMIT_CROSS_DOMAIN"],
        ops["LNP64_M12_COMMIT_MEDIA_FAULT"],
        ops["LNP64_M12_COMMIT_RAW_AUTHORITY"],
    ]


def check_transition_trace(commits: list[dict], states: list[dict], ops: dict[str, int]) -> None:
    actual = [require_int(c, "op") for c in commits]
    if actual != expected_sequence(ops):
        fail(f"M12 typed commit sequence drifted: {actual} != {expected_sequence(ops)}")
    for index, (commit, state) in enumerate(zip(commits, states, strict=True)):
        op = require_int(commit, "op")
        if require_int(commit, "op") != require_int(state, "op") or require_int(commit, "status") != require_int(state, "status"):
            fail(f"M12 commit {index} op/status drifted from state projection")
        # Every commit is bound to a real object/barrier in a real domain.
        if require_int(commit, "object_id") == 0 or require_int(commit, "domain_id") == 0:
            fail(f"M12 commit {index} has unbound object/domain")
        if require_int(commit, "object_generation") == 0 or require_int(commit, "barrier_id") == 0:
            fail(f"M12 commit {index} has zero object generation/barrier id")
        # SG-MEM: raw block-device authority is never exposed in any projection.
        if require_int(state, "no_raw_device_authority") != 1:
            fail(f"M12 commit {index} exposed raw device authority")
        if op == ops["LNP64_M12_COMMIT_BOOT_IMAGE"]:
            if require_int(commit, "status") != ERR_OK or require_int(state, "boot_image_visible") != 1:
                fail(f"M12 boot-image commit {index} was not visible")
        elif op == ops["LNP64_M12_COMMIT_BLOCK_WRITE"]:
            if require_int(commit, "status") != ERR_OK or require_int(state, "block_write_completed") != 1:
                fail(f"M12 block-write commit {index} did not complete")
            if require_int(state, "block_object_authorized") != 1:
                fail(f"M12 block-write commit {index} wrote without an authorized object")
            if require_int(commit, "data_value") == 0:
                fail(f"M12 block-write commit {index} wrote a null payload")
        elif op == ops["LNP64_M12_COMMIT_BARRIER"]:
            if require_int(commit, "status") != ERR_OK or require_int(state, "storage_barrier_issued") != 1:
                fail(f"M12 barrier commit {index} was not issued")
            if require_int(state, "storage_barrier_quiescent") != 1:
                fail(f"M12 barrier commit {index} was not quiescent")
        elif op == ops["LNP64_M12_COMMIT_STALE_OBJECT"]:
            if require_int(commit, "status") != ERR_EREVOKED or require_int(state, "stale_object_rejected") != 1:
                fail(f"M12 stale-object commit {index} was not rejected as revoked")
        elif op == ops["LNP64_M12_COMMIT_CROSS_DOMAIN"]:
            if require_int(commit, "status") != ERR_EPERM or require_int(state, "cross_domain_rejected") != 1:
                fail(f"M12 cross-domain commit {index} was not denied")
        elif op == ops["LNP64_M12_COMMIT_MEDIA_FAULT"]:
            if require_int(commit, "status") != ERR_EIO or require_int(state, "media_fault_terminal") != 1:
                fail(f"M12 media-fault commit {index} was not terminal")
            # By the media fault the rejecting invariants must already hold.
            if require_int(state, "stale_object_rejected") != 1 or require_int(state, "cross_domain_rejected") != 1:
                fail(f"M12 media-fault commit {index} preceded the rejection invariants")
            if require_int(state, "counts_exact") != 1:
                fail(f"M12 media-fault commit {index} did not reach exact counts")
        elif op == ops["LNP64_M12_COMMIT_RAW_AUTHORITY"]:
            if require_int(commit, "status") != ERR_OK:
                fail(f"M12 raw-authority commit {index} did not retire cleanly")
            if require_int(state, "counts_exact") != 1:
                fail(f"M12 raw-authority commit {index} did not have exact counts")
        else:
            fail(f"M12 commit {index} has unknown op {op}")


def sha256_json(data: object) -> str:
    payload = json.dumps(data, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def build_witness(commits, commit_bits, states, state_bits, commit_fields, commit_widths, state_fields, state_widths) -> dict:
    records = []
    for index, (commit, cbits, state, sbits) in enumerate(zip(commits, commit_bits, states, state_bits, strict=True)):
        records.append(
            {
                "index": index,
                "op": commit["op"],
                "status": commit["status"],
                "commit": {f: commit[f] for f in commit_fields},
                "commit_bits": cbits,
                "state": {f: state[f] for f in state_fields},
                "state_bits": sbits,
            }
        )
    artifact = {
        "schema": WITNESS_SCHEMA,
        "commit_schema": {"fields": list(commit_fields), "widths": list(commit_widths), "width": sum(commit_widths)},
        "state_schema": {"fields": list(state_fields), "widths": list(state_widths), "width": sum(state_widths)},
        "record_count": len(records),
        "records": records,
    }
    artifact["records_sha256"] = sha256_json(records)
    return artifact


def main() -> int:
    commit_fields, commit_widths, state_fields, state_widths, ops = load_schema()
    output = run_m12_gate()
    commits = parse_json_records(output, "TTRACE_M12 ", COMMIT_NAME, commit_fields)
    commit_bits = parse_bit_records(output, "TTRACE_M12_BITS ", COMMIT_BITS_NAME, sum(commit_widths))
    states = parse_json_records(output, "TTRACE_M12_STATE ", STATE_NAME, state_fields)
    state_bits = parse_bit_records(output, "TTRACE_M12_STATE_BITS ", STATE_BITS_NAME, sum(state_widths))
    check_bits(commits, commit_bits, commit_fields, commit_widths, "M12 typed commit")
    check_bits(states, state_bits, state_fields, state_widths, "M12 state projection")
    if len(commits) != len(states):
        fail(f"M12 commit count {len(commits)} != state projection count {len(states)}")
    check_transition_trace(commits, states, ops)

    witness_out = os.environ.get("LNP64_RTL_M12_WITNESS_OUT")
    if witness_out:
        artifact = build_witness(commits, commit_bits, states, state_bits, commit_fields, commit_widths, state_fields, state_widths)
        path = Path(witness_out)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print("rtl m12 typed commit trace ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

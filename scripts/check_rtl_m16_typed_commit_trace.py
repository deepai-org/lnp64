#!/usr/bin/env python3
"""Check the M16 RTL typed unified-endpoint commit trace.

Follows the M14/M15 typed-trace pattern for the endpoint (queue) engine. The
lnp64_m16_endpoint engine emits a schema-owned packed commit
(lnp64_m16_endpoint_commit_t) and state projection (lnp64_m16_state_projection_t)
per retired endpoint op: create, send/recv (framing), send-on-full (EAGAIN),
recv-on-empty (EAGAIN), oversize (EMSGSIZE), cap-send (resolve-against-sender +
install-no-amplify), cap-reject (out-of-range), and notify (Register edge). This
checker decodes the packed bit vectors against the shared schema, verifies the
seed-0 commit op sequence, and checks the four EP-F invariant classes per op:
bounded (depth <= capacity, drain bounded), fail-closed (full/empty -> EAGAIN,
oversize -> EMSGSIZE), cap-safety (sender-only resolve, no amplify, reject
out-of-range), and framing (one send = one message = one recv; notify edge).
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
DEFAULT_M16_TRACE_LOG = Path("/tmp/lnp64_rtl_m16_typed_commit.log")

COMMIT_NAME = "m16_endpoint_commit"
STATE_NAME = "m16_state_projection"
COMMIT_BITS_NAME = "m16_endpoint_commit_bits"
STATE_BITS_NAME = "m16_state_projection_bits"
COMMIT_RECORD = "lnp64_m16_endpoint_commit_t"
STATE_RECORD = "lnp64_m16_state_projection_t"
OP_ENUM = "lnp64_m16_endpoint_op_e"
BACKING_ENUM = "lnp64_m16_backing_e"

WITNESS_SCHEMA = "lnp64_m16_endpoint_refinement_witness_v1"

ERR_OK = 0
ERR_EBADF = 9
ERR_EAGAIN = 11
ERR_EMSGSIZE = 90


def fail(message: str) -> None:
    raise SystemExit(f"M16 typed commit check failed: {message}")


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


def load_enum(schema: dict, name: str) -> dict[str, int]:
    values = {}
    for entry in schema["enums"][name]:
        key, value = entry.split("=", maxsplit=1)
        values[key] = parse_sv_int(value)
    return values


def load_schema():
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    commit = [parse_schema_field(e) for e in schema["records"][COMMIT_RECORD]]
    state = [parse_schema_field(e) for e in schema["records"][STATE_RECORD]]
    ops = load_enum(schema, OP_ENUM)
    backings = load_enum(schema, BACKING_ENUM)
    commit_fields = tuple(n for n, _ in commit)
    commit_widths = tuple(w for _, w in commit)
    state_fields = tuple(n for n, _ in state)
    state_widths = tuple(w for _, w in state)
    return commit_fields, commit_widths, state_fields, state_widths, ops, backings


def run_m16_gate() -> str:
    log_path = Path(os.environ.get("LNP64_M16_TYPED_COMMIT_LOG", DEFAULT_M16_TRACE_LOG))
    if os.environ.get("LNP64_M16_TYPED_COMMIT_USE_EXISTING") == "1":
        try:
            return log_path.read_text(encoding="utf-8")
        except OSError as exc:
            fail(f"missing existing M16 typed commit log {log_path}: {exc}")
    proc = subprocess.run(
        ["bash", "scripts/run_rtl_m16.sh"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if proc.returncode != 0:
        print(proc.stdout, end="")
        fail(f"scripts/run_rtl_m16.sh exited with {proc.returncode}")
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
        ops["LNP64_M16_COMMIT_CREATE"],
        ops["LNP64_M16_COMMIT_SEND"],
        ops["LNP64_M16_COMMIT_RECV"],
        ops["LNP64_M16_COMMIT_SEND"],
        ops["LNP64_M16_COMMIT_SEND"],
        ops["LNP64_M16_COMMIT_SEND_FULL"],
        ops["LNP64_M16_COMMIT_RECV"],
        ops["LNP64_M16_COMMIT_RECV"],
        ops["LNP64_M16_COMMIT_RECV_EMPTY"],
        ops["LNP64_M16_COMMIT_OVERSIZE"],
        ops["LNP64_M16_COMMIT_CAP_SEND"],
        ops["LNP64_M16_COMMIT_CAP_REJECT"],
        ops["LNP64_M16_COMMIT_NOTIFY"],
    ]


def check_transition_trace(commits: list[dict], states: list[dict], ops: dict[str, int], backings: dict[str, int]) -> None:
    actual = [require_int(c, "op") for c in commits]
    if actual != expected_sequence(ops):
        fail(f"M16 typed commit sequence drifted: {actual} != {expected_sequence(ops)}")
    for index, (commit, state) in enumerate(zip(commits, states, strict=True)):
        op = require_int(commit, "op")
        if require_int(commit, "op") != require_int(state, "op") or require_int(commit, "status") != require_int(state, "status"):
            fail(f"M16 commit {index} op/status drifted from state projection")
        # Every commit is bound to a real endpoint and scoped to real domains.
        if require_int(commit, "endpoint_id") == 0 or require_int(commit, "endpoint_gen") == 0:
            fail(f"M16 commit {index} has unbound endpoint id/generation")
        if require_int(commit, "sender_domain_id") == 0 or require_int(commit, "receiver_domain_id") == 0:
            fail(f"M16 commit {index} has unscoped sender/receiver domain")
        # (a) bounded: depth never exceeds capacity, drain bounded by capacity.
        if require_int(commit, "depth") > require_int(commit, "capacity"):
            fail(f"M16 commit {index} depth exceeded capacity (unbounded)")
        if require_int(state, "bounded_depth_le_capacity") != 1 or require_int(state, "drain_bounded_by_capacity") != 1:
            fail(f"M16 commit {index} bounded invariant not held")
        # (b) nothing blocks outside an explicit wait.
        if require_int(state, "no_block_except_wait") != 1:
            fail(f"M16 commit {index} blocked outside wait")

        if op == ops["LNP64_M16_COMMIT_CREATE"]:
            if require_int(commit, "status") != ERR_OK or require_int(commit, "backing") != backings["LNP64_M16_BACKING_MEMORY"]:
                fail(f"M16 create commit {index} not a clean Memory-backed create")
            if require_int(commit, "depth") != 0:
                fail(f"M16 create commit {index} did not start empty")
        elif op == ops["LNP64_M16_COMMIT_SEND"]:
            if require_int(commit, "status") != ERR_OK or require_int(commit, "backing") != backings["LNP64_M16_BACKING_MEMORY"]:
                fail(f"M16 send commit {index} did not enqueue cleanly")
        elif op == ops["LNP64_M16_COMMIT_RECV"]:
            if require_int(commit, "status") != ERR_OK:
                fail(f"M16 recv commit {index} did not dequeue cleanly")
            if require_int(state, "framing_one_send_one_recv") != 1:
                fail(f"M16 recv commit {index} did not establish one-send/one-recv framing")
        elif op == ops["LNP64_M16_COMMIT_SEND_FULL"]:
            # fail-closed: a send on a full queue is EAGAIN, never a silent drop,
            # and depth stays at capacity (bounded, not exceeded).
            if require_int(commit, "status") != ERR_EAGAIN or require_int(state, "full_fails_closed") != 1:
                fail(f"M16 send-full commit {index} was not an explicit EAGAIN")
            if require_int(commit, "depth") != require_int(commit, "capacity"):
                fail(f"M16 send-full commit {index} did not hold depth at capacity")
        elif op == ops["LNP64_M16_COMMIT_RECV_EMPTY"]:
            if require_int(commit, "status") != ERR_EAGAIN or require_int(state, "empty_fails_closed") != 1:
                fail(f"M16 recv-empty commit {index} was not an explicit EAGAIN")
            if require_int(commit, "depth") != 0:
                fail(f"M16 recv-empty commit {index} did not hold an empty queue")
        elif op == ops["LNP64_M16_COMMIT_OVERSIZE"]:
            if require_int(commit, "status") != ERR_EMSGSIZE or require_int(state, "oversize_fails_closed") != 1:
                fail(f"M16 oversize commit {index} was not an explicit EMSGSIZE")
        elif op == ops["LNP64_M16_COMMIT_CAP_SEND"]:
            # cap-safety: cap resolved against the sender table and installed
            # into the receiver's with no amplification.
            if require_int(commit, "status") != ERR_OK:
                fail(f"M16 cap-send commit {index} did not send cleanly")
            if require_int(commit, "caps_resolved") < 1 or require_int(commit, "caps_installed") < 1:
                fail(f"M16 cap-send commit {index} did not resolve+install a cap")
            if require_int(commit, "caps_installed") > require_int(commit, "caps_resolved"):
                fail(f"M16 cap-send commit {index} installed more caps than it resolved")
            if require_int(state, "caps_resolve_sender_only") != 1 or require_int(state, "install_no_amplify") != 1:
                fail(f"M16 cap-send commit {index} broke cap-safety (resolve/amplify)")
        elif op == ops["LNP64_M16_COMMIT_CAP_REJECT"]:
            # cap-safety: an out-of-range / revoked handle is rejected and
            # nothing is installed.
            if require_int(commit, "status") != ERR_EBADF or require_int(state, "caps_reject_out_of_range") != 1:
                fail(f"M16 cap-reject commit {index} did not reject the bad handle")
            if require_int(commit, "caps_installed") != 0:
                fail(f"M16 cap-reject commit {index} installed a rejected cap")
        elif op == ops["LNP64_M16_COMMIT_NOTIFY"]:
            # framing: an empty send to a Register-backed endpoint raises its edge.
            if require_int(commit, "status") != ERR_OK or require_int(commit, "backing") != backings["LNP64_M16_BACKING_REGISTER"]:
                fail(f"M16 notify commit {index} not a clean Register-backed notify")
            if require_int(state, "notify_raises_register_edge") != 1:
                fail(f"M16 notify commit {index} did not raise the register edge")
            # By the terminal notify all four EP-F invariant classes must hold.
            for flag in (
                "full_fails_closed",
                "empty_fails_closed",
                "oversize_fails_closed",
                "caps_resolve_sender_only",
                "caps_reject_out_of_range",
                "install_no_amplify",
                "framing_one_send_one_recv",
            ):
                if require_int(state, flag) != 1:
                    fail(f"M16 notify commit {index} preceded invariant {flag}")
            if require_int(state, "counts_exact") != 1:
                fail(f"M16 notify commit {index} did not reach exact counts")
        else:
            fail(f"M16 commit {index} has unknown op {op}")


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
    commit_fields, commit_widths, state_fields, state_widths, ops, backings = load_schema()
    output = run_m16_gate()
    commits = parse_json_records(output, "TTRACE_M16 ", COMMIT_NAME, commit_fields)
    commit_bits = parse_bit_records(output, "TTRACE_M16_BITS ", COMMIT_BITS_NAME, sum(commit_widths))
    states = parse_json_records(output, "TTRACE_M16_STATE ", STATE_NAME, state_fields)
    state_bits = parse_bit_records(output, "TTRACE_M16_STATE_BITS ", STATE_BITS_NAME, sum(state_widths))
    check_bits(commits, commit_bits, commit_fields, commit_widths, "M16 typed commit")
    check_bits(states, state_bits, state_fields, state_widths, "M16 state projection")
    if len(commits) != len(states):
        fail(f"M16 commit count {len(commits)} != state projection count {len(states)}")
    check_transition_trace(commits, states, ops, backings)

    witness_out = os.environ.get("LNP64_RTL_M16_WITNESS_OUT")
    if witness_out:
        artifact = build_witness(commits, commit_bits, states, state_bits, commit_fields, commit_widths, state_fields, state_widths)
        path = Path(witness_out)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print("rtl m16 typed commit trace ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

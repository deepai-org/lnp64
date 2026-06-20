#!/usr/bin/env python3
"""Check the M7 RTL typed scheduler/wakeup commit trace.

This is a narrow follow-on to the M1 refinement pattern.  It checks the seed-0
M7 RTL typed commit/state projection stream against the current Lean
`M7TransitionInvariantModel` transition shape: cmpxchg success/fail, futex
wait/wake, wake consumption, timer wait/expire, and stale-address rejection.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "rtl/schema/lnp64_shared_schema.json"
RTL_PKG = ROOT / "rtl/include/lnp64_pkg.sv"
RTL_M7_TB = ROOT / "rtl/sim/lnp64_m7_tb.sv"
DEFAULT_M7_TRACE_LOG = Path("/tmp/lnp64_rtl_m7_typed_commit.log")

ERR_OK = 0
ERR_EAGAIN = 11
ERR_EREVOKED = 122

LOC_RUNNABLE = 1
LOC_PARKED = 3
M7_TID = 2

COMMIT_RECORD = "lnp64_m7_sched_commit_t"
STATE_RECORD = "lnp64_m7_state_projection_t"

WITNESS_SCHEMA = "lnp64_m7_sched_refinement_witness_v1"
COMMIT_NAME = "m7_sched_commit"
STATE_NAME = "m7_state_projection"
COMMIT_BITS_NAME = "m7_sched_commit_bits"
STATE_BITS_NAME = "m7_state_projection_bits"

COMMIT_FIELDS = (
    "op",
    "status",
    "tid",
    "before_location",
    "after_location",
    "wait_generation",
    "address_generation",
)

STATE_FIELDS = (
    "op",
    "status",
    "tid",
    "location",
    "wait_generation",
    "atomic_word",
    "atomic_count",
    "cmpxchg_failure_explicit",
    "address_generation",
    "stale_address_generation",
    "domain_budget",
    "wait_cost",
    "wake_pending",
    "futex_wake_delivered",
    "timer_wake_delivered",
    "stale_address_rejected",
)


@dataclass(frozen=True)
class Ops:
    cmpxchg_success: int
    cmpxchg_fail: int
    futex_wait: int
    futex_wake: int
    timer_wait: int
    timer_expire: int
    consume_wake: int
    reject_stale_address: int

    @property
    def expected_sequence(self) -> list[int]:
        return [
            self.cmpxchg_success,
            self.cmpxchg_fail,
            self.futex_wait,
            self.futex_wake,
            self.consume_wake,
            self.timer_wait,
            self.timer_expire,
            self.reject_stale_address,
        ]


@dataclass
class State:
    atomic_word: int = 0
    atomic_count: int = 0
    cmpxchg_failure_explicit: bool = False
    location: int = LOC_RUNNABLE
    wait_generation: int = 1
    address_generation: int = 1
    stale_address_generation: int = 0
    domain_budget: int = 1
    wait_cost: int = 1
    wake_pending: bool = False
    futex_wake_delivered: bool = False
    timer_wake_delivered: bool = False
    stale_address_rejected: bool = False

    def as_projection(self, op: int, status: int) -> dict[str, int]:
        return {
            "op": op,
            "status": status,
            "tid": M7_TID,
            "location": self.location,
            "wait_generation": self.wait_generation,
            "atomic_word": self.atomic_word,
            "atomic_count": self.atomic_count,
            "cmpxchg_failure_explicit": int(self.cmpxchg_failure_explicit),
            "address_generation": self.address_generation,
            "stale_address_generation": self.stale_address_generation,
            "domain_budget": self.domain_budget,
            "wait_cost": self.wait_cost,
            "wake_pending": int(self.wake_pending),
            "futex_wake_delivered": int(self.futex_wake_delivered),
            "timer_wake_delivered": int(self.timer_wake_delivered),
            "stale_address_rejected": int(self.stale_address_rejected),
        }


def fail(message: str) -> None:
    raise SystemExit(f"rtl m7 typed commit trace check failed: {message}")


def parse_sv_int(value: str) -> int:
    text = re.sub(r"_", "", value.strip())
    match = re.fullmatch(r"(?:(?P<bits>\d+)'(?P<base>[dhb]))?(?P<digits>[0-9a-fA-F]+)", text)
    if not match:
        fail(f"could not parse SV integer {value!r}")
    base = {"d": 10, "h": 16, "b": 2, None: 10}[match.group("base")]
    return int(match.group("digits"), base)


def parse_schema_field(entry: str) -> tuple[str, int]:
    name, raw_width = entry.split(":", 1)
    return name, int(raw_width)


def parse_sv_packed_struct_fields(source: str, typedef_name: str) -> tuple[tuple[str, int], ...]:
    match = re.search(
        rf"(?ms)typedef\s+struct\s+packed\s*\{{(?P<body>[^}}]*)\}}\s*{re.escape(typedef_name)}\s*;",
        source,
    )
    if not match:
        fail(f"RTL package is missing packed typedef {typedef_name}")
    fields: list[tuple[str, int]] = []
    for raw_line in match.group("body").splitlines():
        line = raw_line.split("//", 1)[0].strip()
        if not line:
            continue
        field_match = re.fullmatch(
            r"logic(?:\s*\[\s*(?P<msb>[0-9]+)\s*:\s*(?P<lsb>[0-9]+)\s*\])?\s+"
            r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;",
            line,
        )
        if not field_match:
            fail(f"could not parse field in RTL packed typedef {typedef_name}: {raw_line!r}")
        msb = field_match.group("msb")
        lsb = field_match.group("lsb")
        width = 1 if msb is None else abs(int(msb) - int(lsb)) + 1
        fields.append((field_match.group("name"), width))
    if not fields:
        fail(f"RTL packed typedef {typedef_name} has no parsed fields")
    return tuple(fields)


def check_rtl_packed_typedefs_match_schema_sources(
    pkg_source: str,
    commit_field_specs: tuple[tuple[str, int], ...],
    state_field_specs: tuple[tuple[str, int], ...],
) -> None:
    commit_typedef = parse_sv_packed_struct_fields(pkg_source, COMMIT_RECORD)
    if commit_typedef != commit_field_specs:
        fail(
            f"RTL {COMMIT_RECORD} packed typedef drifted from shared schema: "
            f"{commit_typedef!r} != {commit_field_specs!r}"
        )
    state_typedef = parse_sv_packed_struct_fields(pkg_source, STATE_RECORD)
    if state_typedef != state_field_specs:
        fail(
            f"RTL {STATE_RECORD} packed typedef drifted from shared schema: "
            f"{state_typedef!r} != {state_field_specs!r}"
        )


def check_rtl_packed_typedefs_match_schema(
    commit_field_specs: tuple[tuple[str, int], ...],
    state_field_specs: tuple[tuple[str, int], ...],
) -> None:
    try:
        pkg_source = RTL_PKG.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"could not read RTL package: {exc}")
    check_rtl_packed_typedefs_match_schema_sources(
        pkg_source,
        commit_field_specs,
        state_field_specs,
    )


def require_trace_display_payload_source(
    tb_source: str,
    marker: str,
    source_signal: str,
    message: str,
) -> None:
    marker_index = tb_source.find(marker)
    if marker_index < 0:
        fail(message)
    display_end = tb_source.find(");", marker_index)
    if display_end < 0:
        fail(message)
    display_call = tb_source[marker_index:display_end]
    if re.search(rf"\b{re.escape(source_signal)}\b", display_call) is None:
        fail(message)


def check_m7_testbench_trace_source_contract() -> None:
    try:
        tb_source = RTL_M7_TB.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"could not read M7 testbench source: {exc}")
    required_bit_width_sources = (
        "$bits(lnp64_m7_sched_commit_t)",
        "$bits(lnp64_m7_state_projection_t)",
    )
    missing_bit_width_sources = [
        source
        for source in required_bit_width_sources
        if source not in tb_source
    ]
    if missing_bit_width_sources:
        fail(
            "M7 testbench no longer emits schema-owned packed bit widths: "
            f"{missing_bit_width_sources}"
        )
    require_trace_display_payload_source(
        tb_source,
        "TTRACE_M7_BITS",
        "typed_commit",
        "M7 testbench no longer emits packed commit bits from typed_commit",
    )
    require_trace_display_payload_source(
        tb_source,
        "TTRACE_M7_STATE_BITS",
        "typed_state_projection",
        "M7 testbench no longer emits packed state bits from typed_state_projection",
    )


def load_schema() -> tuple[tuple[str, ...], tuple[int, ...], tuple[str, ...], tuple[int, ...], Ops]:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    records = schema.get("records", {})
    enums = schema.get("enums", {})
    commit_specs = tuple(parse_schema_field(entry) for entry in records.get(COMMIT_RECORD, []))
    state_specs = tuple(parse_schema_field(entry) for entry in records.get(STATE_RECORD, []))
    commit_fields = tuple(name for name, _width in commit_specs)
    state_fields = tuple(name for name, _width in state_specs)
    if commit_fields != COMMIT_FIELDS:
        fail(f"M7 commit schema fields drifted: {commit_fields!r} != {COMMIT_FIELDS!r}")
    if state_fields != STATE_FIELDS:
        fail(f"M7 state projection schema fields drifted: {state_fields!r} != {STATE_FIELDS!r}")
    check_rtl_packed_typedefs_match_schema(commit_specs, state_specs)

    enum_entries = enums.get("lnp64_m7_commit_op_e", [])
    enum_values = {entry.split("=", 1)[0]: parse_sv_int(entry.split("=", 1)[1]) for entry in enum_entries}
    try:
        ops = Ops(
            cmpxchg_success=enum_values["LNP64_M7_COMMIT_CMPXCHG_SUCCESS"],
            cmpxchg_fail=enum_values["LNP64_M7_COMMIT_CMPXCHG_FAIL"],
            futex_wait=enum_values["LNP64_M7_COMMIT_FUTEX_WAIT"],
            futex_wake=enum_values["LNP64_M7_COMMIT_FUTEX_WAKE"],
            timer_wait=enum_values["LNP64_M7_COMMIT_TIMER_WAIT"],
            timer_expire=enum_values["LNP64_M7_COMMIT_TIMER_EXPIRE"],
            consume_wake=enum_values["LNP64_M7_COMMIT_CONSUME_WAKE"],
            reject_stale_address=enum_values["LNP64_M7_COMMIT_REJECT_STALE_ADDRESS"],
        )
    except KeyError as exc:
        fail(f"M7 op enum is missing {exc.args[0]}")
    return (
        commit_fields,
        tuple(width for _name, width in commit_specs),
        state_fields,
        tuple(width for _name, width in state_specs),
        ops,
    )


def run_m7_gate() -> str:
    log_path = Path(os.environ.get("LNP64_M7_TYPED_COMMIT_LOG", DEFAULT_M7_TRACE_LOG))
    if os.environ.get("LNP64_M7_TYPED_COMMIT_USE_EXISTING") == "1":
        try:
            return log_path.read_text(encoding="utf-8")
        except OSError as exc:
            fail(f"missing existing M7 typed commit log {log_path}: {exc}")

    env = os.environ.copy()
    env["LNP64_COSIM_SEEDS"] = os.environ.get("LNP64_M7_TYPED_COMMIT_SEEDS", "0")
    proc = subprocess.run(
        ["bash", "scripts/run_rtl_m7.sh"],
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if proc.returncode != 0:
        print(proc.stdout, end="")
        fail(f"scripts/run_rtl_m7.sh exited with {proc.returncode}")
    try:
        log_path.write_text(proc.stdout, encoding="utf-8")
    except OSError as exc:
        fail(f"could not write M7 typed commit log {log_path}: {exc}")
    return proc.stdout


def require_int(record: dict[str, int | str], key: str) -> int:
    value = record.get(key)
    if not isinstance(value, int):
        fail(f"record {record.get('record')} has non-integer {key}: {value!r}")
    return value


def parse_json_records(output: str, prefix: str, record_name: str, fields: tuple[str, ...]) -> list[dict[str, int | str]]:
    parsed: list[dict[str, int | str]] = []
    for line in output.splitlines():
        if not line.startswith(prefix):
            continue
        payload = line.removeprefix(prefix)
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid JSON record {payload!r}: {exc}")
        if record.get("record") != record_name:
            fail(f"unexpected record type {record.get('record')!r}")
        actual_fields = tuple(key for key in record if key != "record")
        if actual_fields != fields:
            fail(f"{record_name} fields drifted: {actual_fields!r} != {fields!r}")
        for field in fields:
            require_int(record, field)
        parsed.append(record)
    if not parsed:
        fail(f"no {prefix.strip()} records emitted")
    return parsed


def parse_bit_records(output: str, prefix: str, record_name: str, expected_width: int) -> list[str]:
    parsed: list[str] = []
    for line in output.splitlines():
        if not line.startswith(prefix):
            continue
        payload = line.removeprefix(prefix)
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid packed bit record {payload!r}: {exc}")
        if record.get("record") != record_name:
            fail(f"unexpected packed bit record type {record.get('record')!r}")
        width = record.get("width")
        if width != expected_width:
            fail(
                f"packed bit record {record_name} width drifted from schema: "
                f"{width!r} != {expected_width}"
            )
        bits = record.get("bits")
        if not isinstance(bits, str) or not re.fullmatch(r"[0-9a-fA-F]+", bits):
            fail(f"packed bit record {record_name} has invalid bits {bits!r}")
        parsed.append(bits)
    if not parsed:
        fail(f"no {prefix.strip()} records emitted")
    return parsed


def decode_packed_bits(bits: str, fields: tuple[str, ...], widths: tuple[int, ...]) -> dict[str, int]:
    total_width = sum(widths)
    value = int(bits, 16)
    if value >= (1 << total_width):
        fail(f"packed bits exceed schema width {total_width}: 0x{bits}")
    decoded: dict[str, int] = {}
    shift = total_width
    for field, width in zip(fields, widths, strict=True):
        shift -= width
        decoded[field] = (value >> shift) & ((1 << width) - 1)
    return decoded


def check_bits(
    records: list[dict[str, int | str]],
    bits: list[str],
    fields: tuple[str, ...],
    widths: tuple[int, ...],
    label: str,
) -> None:
    if len(records) != len(bits):
        fail(f"{label} packed bit count {len(bits)} != record count {len(records)}")
    for index, (record, bit_record) in enumerate(zip(records, bits, strict=True)):
        decoded = decode_packed_bits(bit_record, fields, widths)
        for field in fields:
            actual = require_int(record, field)
            if decoded[field] != actual:
                fail(f"{label} packed decode drift at {index} field {field}: {decoded[field]} != {actual}")


def require_commit(record: dict[str, int | str], op: int, status: int, before: int, after: int) -> None:
    if require_int(record, "op") != op:
        fail(f"unexpected op: {require_int(record, 'op')} != {op}")
    if require_int(record, "status") != status:
        fail(f"unexpected status for op {op}: {require_int(record, 'status')} != {status}")
    if require_int(record, "tid") != M7_TID:
        fail(f"unexpected tid for op {op}")
    if require_int(record, "before_location") != before:
        fail(f"unexpected before location for op {op}")
    if require_int(record, "after_location") != after:
        fail(f"unexpected after location for op {op}")


def check_projection(record: dict[str, int | str], expected: dict[str, int], index: int) -> None:
    for field, value in expected.items():
        actual = require_int(record, field)
        if actual != value:
            fail(f"state projection {index} field {field} drifted: {actual} != {value}")


def check_transition_trace(
    commits: list[dict[str, int | str]],
    states: list[dict[str, int | str]],
    ops: Ops,
) -> None:
    actual_sequence = [require_int(record, "op") for record in commits]
    if actual_sequence != ops.expected_sequence:
        fail(f"M7 typed commit sequence drifted: {actual_sequence} != {ops.expected_sequence}")
    state = State()
    for index, (commit, projection) in enumerate(zip(commits, states, strict=True)):
        op = require_int(commit, "op")
        if op == ops.cmpxchg_success:
            require_commit(commit, op, ERR_OK, LOC_RUNNABLE, LOC_RUNNABLE)
            if state.atomic_count != 0:
                fail("cmpxchgSuccess precondition failed")
            state.atomic_word = 1
            state.atomic_count = 1
        elif op == ops.cmpxchg_fail:
            require_commit(commit, op, ERR_EAGAIN, LOC_RUNNABLE, LOC_RUNNABLE)
            if state.atomic_count != 1 or state.atomic_word != 1:
                fail("cmpxchgFail precondition failed")
            state.atomic_count = 2
            state.cmpxchg_failure_explicit = True
        elif op == ops.futex_wait:
            require_commit(commit, op, ERR_OK, LOC_RUNNABLE, LOC_PARKED)
            if state.wake_pending:
                fail("futexWait precondition failed")
            state.location = LOC_PARKED
            state.wait_generation = state.address_generation
        elif op == ops.futex_wake:
            require_commit(commit, op, ERR_OK, LOC_PARKED, LOC_RUNNABLE)
            if state.location != LOC_PARKED or state.wait_generation != state.address_generation:
                fail("futexWake precondition failed")
            state.location = LOC_RUNNABLE
            state.wake_pending = True
            state.futex_wake_delivered = True
        elif op == ops.consume_wake:
            require_commit(commit, op, ERR_OK, LOC_RUNNABLE, LOC_RUNNABLE)
            if not state.wake_pending:
                fail("consumeWake precondition failed")
            state.wake_pending = False
        elif op == ops.timer_wait:
            require_commit(commit, op, ERR_OK, LOC_RUNNABLE, LOC_PARKED)
            if state.wake_pending:
                fail("timerWait precondition failed")
            state.location = LOC_PARKED
            state.wait_generation = state.address_generation
        elif op == ops.timer_expire:
            require_commit(commit, op, ERR_OK, LOC_PARKED, LOC_RUNNABLE)
            if state.location != LOC_PARKED or state.wait_generation != state.address_generation:
                fail("timerExpire precondition failed")
            state.location = LOC_RUNNABLE
            state.wake_pending = True
            state.timer_wake_delivered = True
        elif op == ops.reject_stale_address:
            require_commit(commit, op, ERR_EREVOKED, LOC_RUNNABLE, LOC_RUNNABLE)
            if state.stale_address_generation == state.address_generation:
                fail("rejectStaleAddress precondition failed")
            state.stale_address_rejected = True
        else:
            fail(f"unknown op {op}")
        check_projection(projection, state.as_projection(op, require_int(commit, "status")), index)


def sha256_json(data: object) -> str:
    payload = json.dumps(data, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def build_m7_witness_artifact(
    commits: list[dict[str, int | str]],
    commit_bits: list[str],
    states: list[dict[str, int | str]],
    state_bits: list[str],
    commit_fields: tuple[str, ...],
    commit_widths: tuple[int, ...],
    state_fields: tuple[str, ...],
    state_widths: tuple[int, ...],
    ops: Ops,
) -> dict:
    records = []
    for index, (commit, cbits, state, sbits) in enumerate(
        zip(commits, commit_bits, states, state_bits, strict=True)
    ):
        records.append(
            {
                "index": index,
                "op": commit["op"],
                "status": commit["status"],
                "commit": {field: commit[field] for field in commit_fields},
                "commit_bits": cbits,
                "state": {field: state[field] for field in state_fields},
                "state_bits": sbits,
            }
        )
    artifact = {
        "schema": WITNESS_SCHEMA,
        "commit_schema": {
            "fields": list(commit_fields),
            "widths": list(commit_widths),
            "width": sum(commit_widths),
        },
        "state_schema": {
            "fields": list(state_fields),
            "widths": list(state_widths),
            "width": sum(state_widths),
        },
        "ops": {
            "cmpxchg_success": ops.cmpxchg_success,
            "cmpxchg_fail": ops.cmpxchg_fail,
            "futex_wait": ops.futex_wait,
            "futex_wake": ops.futex_wake,
            "timer_wait": ops.timer_wait,
            "timer_expire": ops.timer_expire,
            "consume_wake": ops.consume_wake,
            "reject_stale_address": ops.reject_stale_address,
        },
        "record_count": len(records),
        "records": records,
    }
    artifact["records_sha256"] = sha256_json(records)
    return artifact


def write_m7_witness_artifact(output_path: str, artifact: dict) -> None:
    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    commit_fields, commit_widths, state_fields, state_widths, ops = load_schema()
    check_m7_testbench_trace_source_contract()
    output = run_m7_gate()
    commits = parse_json_records(output, "TTRACE_M7 ", COMMIT_NAME, commit_fields)
    commit_bits = parse_bit_records(output, "TTRACE_M7_BITS ", COMMIT_BITS_NAME, sum(commit_widths))
    states = parse_json_records(output, "TTRACE_M7_STATE ", STATE_NAME, state_fields)
    state_bits = parse_bit_records(output, "TTRACE_M7_STATE_BITS ", STATE_BITS_NAME, sum(state_widths))
    check_bits(commits, commit_bits, commit_fields, commit_widths, "M7 typed commit")
    check_bits(states, state_bits, state_fields, state_widths, "M7 state projection")
    if len(commits) != len(states):
        fail(f"M7 commit count {len(commits)} != state projection count {len(states)}")
    check_transition_trace(commits, states, ops)

    witness_out = os.environ.get("LNP64_RTL_M7_WITNESS_OUT")
    if witness_out:
        artifact = build_m7_witness_artifact(
            commits,
            commit_bits,
            states,
            state_bits,
            commit_fields,
            commit_widths,
            state_fields,
            state_widths,
            ops,
        )
        write_m7_witness_artifact(witness_out, artifact)

    print("rtl m7 typed commit trace ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

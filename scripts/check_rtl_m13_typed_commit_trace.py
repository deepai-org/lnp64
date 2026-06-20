#!/usr/bin/env python3
"""Check the M13 RTL typed PCIe/IOMMU commit trace.

Follows the M1/M7/M10/M11/M12 typed-trace pattern for the SG-IO PCIe device
slice. The lnp64_m13_pcie_iommu engine emits a schema-owned packed commit
(lnp64_m13_pcie_commit_t) and state projection (lnp64_m13_state_projection_t)
per PCIe transition: device enumeration (BAR capability created), IOMMU-bound
scoped DMA, MSI delivery, unbound bus-master rejection, stale-BAR rejection,
malformed-config rejection, and the raw-authority retirement. This checker
decodes the packed bit vectors against the shared schema, verifies the seed-0
commit op sequence, and checks the authority-relevant per-op invariants --
requester/BAR/domain binding, never-exposed raw PCIe DMA/interrupt authority,
and the rejection invariants holding before the terminal malformed-config fault.
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
DEFAULT_M13_TRACE_LOG = Path("/tmp/lnp64_rtl_m13_typed_commit.log")

COMMIT_NAME = "m13_pcie_commit"
STATE_NAME = "m13_state_projection"
COMMIT_BITS_NAME = "m13_pcie_commit_bits"
STATE_BITS_NAME = "m13_state_projection_bits"
COMMIT_RECORD = "lnp64_m13_pcie_commit_t"
STATE_RECORD = "lnp64_m13_state_projection_t"
OP_ENUM = "lnp64_m13_pcie_op_e"

WITNESS_SCHEMA = "lnp64_m13_pcie_refinement_witness_v1"

ERR_OK = 0
ERR_EPERM = 1
ERR_EIO = 5
ERR_EINVAL = 22
ERR_EREVOKED = 122


def fail(message: str) -> None:
    raise SystemExit(f"M13 typed commit check failed: {message}")


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


def run_m13_gate() -> str:
    log_path = Path(os.environ.get("LNP64_M13_TYPED_COMMIT_LOG", DEFAULT_M13_TRACE_LOG))
    if os.environ.get("LNP64_M13_TYPED_COMMIT_USE_EXISTING") == "1":
        try:
            return log_path.read_text(encoding="utf-8")
        except OSError as exc:
            fail(f"missing existing M13 typed commit log {log_path}: {exc}")
    proc = subprocess.run(
        ["bash", "scripts/run_rtl_m13.sh"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if proc.returncode != 0:
        print(proc.stdout, end="")
        fail(f"scripts/run_rtl_m13.sh exited with {proc.returncode}")
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
        ops["LNP64_M13_COMMIT_ENUMERATE"],
        ops["LNP64_M13_COMMIT_IOMMU_DMA"],
        ops["LNP64_M13_COMMIT_MSI"],
        ops["LNP64_M13_COMMIT_BUS_MASTER"],
        ops["LNP64_M13_COMMIT_STALE_BAR"],
        ops["LNP64_M13_COMMIT_MALFORMED_CONFIG"],
        ops["LNP64_M13_COMMIT_RAW_AUTHORITY"],
    ]


def check_transition_trace(commits: list[dict], states: list[dict], ops: dict[str, int]) -> None:
    actual = [require_int(c, "op") for c in commits]
    if actual != expected_sequence(ops):
        fail(f"M13 typed commit sequence drifted: {actual} != {expected_sequence(ops)}")
    for index, (commit, state) in enumerate(zip(commits, states, strict=True)):
        op = require_int(commit, "op")
        if require_int(commit, "op") != require_int(state, "op") or require_int(commit, "status") != require_int(state, "status"):
            fail(f"M13 commit {index} op/status drifted from state projection")
        # Every commit is bound to a real requester/BAR in a real domain.
        if require_int(commit, "requester_id") == 0 or require_int(commit, "domain_id") == 0:
            fail(f"M13 commit {index} has unbound requester/domain")
        if require_int(commit, "bar_id") == 0 or require_int(commit, "bar_generation") == 0:
            fail(f"M13 commit {index} has zero BAR id/generation")
        # SG-IO: raw PCIe DMA/interrupt authority is never exposed in any projection.
        if require_int(state, "no_raw_pcie_authority") != 1:
            fail(f"M13 commit {index} exposed raw PCIe authority")
        if op == ops["LNP64_M13_COMMIT_ENUMERATE"]:
            if require_int(commit, "status") != ERR_OK or require_int(state, "device_enumerated") != 1:
                fail(f"M13 enumerate commit {index} did not enumerate")
            if require_int(state, "bar_capability_created") != 1:
                fail(f"M13 enumerate commit {index} created no BAR capability")
        elif op == ops["LNP64_M13_COMMIT_IOMMU_DMA"]:
            if require_int(commit, "status") != ERR_OK or require_int(state, "iommu_bound_to_domain") != 1:
                fail(f"M13 iommu-dma commit {index} was not IOMMU-bound to a domain")
            if require_int(state, "scoped_dma_completed") != 1:
                fail(f"M13 iommu-dma commit {index} did not complete a scoped DMA")
            if require_int(commit, "dma_bytes") == 0:
                fail(f"M13 iommu-dma commit {index} moved a null DMA window")
        elif op == ops["LNP64_M13_COMMIT_MSI"]:
            if require_int(commit, "status") != ERR_OK or require_int(state, "msi_event_delivered") != 1:
                fail(f"M13 msi commit {index} did not deliver an MSI event")
        elif op == ops["LNP64_M13_COMMIT_BUS_MASTER"]:
            if require_int(commit, "status") != ERR_EPERM or require_int(state, "unbound_bus_master_rejected") != 1:
                fail(f"M13 bus-master commit {index} did not deny the unbound bus master")
        elif op == ops["LNP64_M13_COMMIT_STALE_BAR"]:
            if require_int(commit, "status") != ERR_EREVOKED or require_int(state, "stale_bar_rejected") != 1:
                fail(f"M13 stale-bar commit {index} was not rejected as revoked")
        elif op == ops["LNP64_M13_COMMIT_MALFORMED_CONFIG"]:
            if require_int(commit, "status") != ERR_EINVAL or require_int(state, "malformed_config_rejected") != 1:
                fail(f"M13 malformed-config commit {index} was not rejected as invalid")
            # By the malformed-config fault the prior rejections must already hold.
            if require_int(state, "unbound_bus_master_rejected") != 1 or require_int(state, "stale_bar_rejected") != 1:
                fail(f"M13 malformed-config commit {index} preceded the rejection invariants")
            if require_int(state, "counts_exact") != 1:
                fail(f"M13 malformed-config commit {index} did not reach exact counts")
        elif op == ops["LNP64_M13_COMMIT_RAW_AUTHORITY"]:
            if require_int(commit, "status") != ERR_OK:
                fail(f"M13 raw-authority commit {index} did not retire cleanly")
            if require_int(state, "counts_exact") != 1:
                fail(f"M13 raw-authority commit {index} did not have exact counts")
        else:
            fail(f"M13 commit {index} has unknown op {op}")


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
    output = run_m13_gate()
    commits = parse_json_records(output, "TTRACE_M13 ", COMMIT_NAME, commit_fields)
    commit_bits = parse_bit_records(output, "TTRACE_M13_BITS ", COMMIT_BITS_NAME, sum(commit_widths))
    states = parse_json_records(output, "TTRACE_M13_STATE ", STATE_NAME, state_fields)
    state_bits = parse_bit_records(output, "TTRACE_M13_STATE_BITS ", STATE_BITS_NAME, sum(state_widths))
    check_bits(commits, commit_bits, commit_fields, commit_widths, "M13 typed commit")
    check_bits(states, state_bits, state_fields, state_widths, "M13 state projection")
    if len(commits) != len(states):
        fail(f"M13 commit count {len(commits)} != state projection count {len(states)}")
    check_transition_trace(commits, states, ops)

    witness_out = os.environ.get("LNP64_RTL_M13_WITNESS_OUT")
    if witness_out:
        artifact = build_witness(commits, commit_bits, states, state_bits, commit_fields, commit_widths, state_fields, state_widths)
        path = Path(witness_out)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print("rtl m13 typed commit trace ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

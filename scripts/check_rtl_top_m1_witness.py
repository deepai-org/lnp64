#!/usr/bin/env python3
"""Offline checker for the lnp64_top M1 refinement witness artifact.

`scripts/run_rtl_top_program_smoke.sh` can emit a
`lnp64_top_m1_refinement_witness_v1` JSON artifact (via
`LNP64_RTL_TOP_M1_WITNESS_OUT`) that captures, for every accepted top-level M1
commit retired through `lnp64_top`, the typed commit, the authority-bearing
pre/post state projections, and their packed bit vectors.

This checker re-validates that artifact without re-running the simulator:

1. the artifact schema, commit/state schema field order, widths, and total
   widths match the shared RTL schema (`rtl/schema/lnp64_shared_schema.json`);
2. the canonical records hash recomputes (artifact integrity);
3. every packed commit/pre/post bit vector decodes back to its stored
   projection fields by the schema (packed-bit faithfulness); and
4. every record satisfies the shared executable M1 refinement step
   (`formal/m1_top_refinement.py`) -- the same relation the producer enforces
   inline -- so the artifact alone witnesses a valid M1 refinement step.

It is executable-level (T2-shape) evidence: it does not yet replace a formal
RTL-to-Lean bit-refinement proof, but it makes the generated witness an
auditable, re-checkable artifact instead of orphaned plumbing.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = ROOT / "rtl/schema/lnp64_shared_schema.json"
DEFAULT_WITNESS = ROOT / "build/lnp64-top-m1-refinement-witness.json"

sys.path.insert(0, str(ROOT / "formal"))
from m1_top_refinement import (  # noqa: E402
    authority_projection_fields,
    check_top_m1_optional_authority_slots,
    check_top_m1_refinement_step,
    decode_packed_bits,
    sha256_json,
)

WITNESS_SCHEMA = "lnp64_top_m1_refinement_witness_v1"
COMMIT_RECORD = "lnp64_m1_cap_commit_t"
STATE_RECORD = "lnp64_m1_state_projection_t"
COMMIT_OP_ENUM = "lnp64_m1_commit_op_e"

# Schema enum name -> executable mirror key used by the shared refinement step.
COMMIT_OP_KEYS = {
    "LNP64_M1_COMMIT_CAP_DUP": "CapDup",
    "LNP64_M1_COMMIT_CAP_DUP_DENIED": "CapDupDenied",
    "LNP64_M1_COMMIT_CAP_SEND": "CapSend",
    "LNP64_M1_COMMIT_CAP_RECV": "CapRecv",
    "LNP64_M1_COMMIT_CAP_REVOKE": "CapRevoke",
    "LNP64_M1_COMMIT_REJECT_STALE": "RejectStale",
    "LNP64_M1_COMMIT_PUSH": "Push",
    "LNP64_M1_COMMIT_PULL": "Pull",
    "LNP64_M1_COMMIT_REJECT_FULL": "RejectFull",
    "LNP64_M1_COMMIT_OBJECT_CREATE": "ObjectCreate",
}


def fail(message: str) -> None:
    raise SystemExit(f"top-level M1 witness check failed: {message}")


def parse_int_literal(value: str) -> int:
    value = value.strip()
    if "'h" in value:
        return int(value.split("'h", maxsplit=1)[1].replace("_", ""), 16)
    if "'d" in value:
        return int(value.split("'d", maxsplit=1)[1].replace("_", ""), 10)
    if value.lower().startswith("0x"):
        return int(value, 16)
    return int(value, 10)


def load_schema() -> dict:
    if not SCHEMA_PATH.exists():
        fail(f"missing shared schema {SCHEMA_PATH}")
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


def load_record_schema(schema: dict, record_name: str) -> tuple[tuple[str, ...], tuple[int, ...]]:
    entries = schema["records"][record_name]
    fields = []
    widths = []
    for entry in entries:
        field, width = entry.split(":", maxsplit=1)
        fields.append(field)
        widths.append(int(width))
    return tuple(fields), tuple(widths)


def load_commit_ops(schema: dict) -> dict[str, int]:
    values = {}
    for entry in schema["enums"][COMMIT_OP_ENUM]:
        name, value = entry.split("=", maxsplit=1)
        values[name] = parse_int_literal(value)
    ops = {}
    for enum_name, key in COMMIT_OP_KEYS.items():
        if enum_name not in values:
            fail(f"shared schema is missing commit op {enum_name}")
        ops[key] = values[enum_name]
    return ops


def require_schema_block(label: str, block: object, fields: tuple[str, ...], widths: tuple[int, ...]) -> None:
    if not isinstance(block, dict):
        fail(f"{label} schema block missing")
    if tuple(block.get("fields", ())) != fields:
        fail(f"{label} schema field order drifted from shared schema")
    if tuple(block.get("widths", ())) != widths:
        fail(f"{label} schema widths drifted from shared schema")
    if block.get("width") != sum(widths):
        fail(f"{label} schema total width drifted from shared schema")


def check_packed(label: str, idx: int, bits: object, projection: dict, fields: tuple[str, ...], widths: tuple[int, ...]) -> None:
    if not isinstance(bits, str):
        fail(f"record {idx} {label} bits missing or non-string")
    decoded = decode_packed_bits(bits, fields, widths)
    for field in fields:
        if field not in projection:
            fail(f"record {idx} {label} projection missing field {field}")
        if decoded[field] != projection[field]:
            fail(
                f"record {idx} {label} packed bits drifted from projection field {field}: "
                f"packed={decoded[field]} json={projection[field]}"
            )
    extra = sorted(set(projection) - set(fields))
    if extra:
        fail(f"record {idx} {label} projection has fields outside schema: {extra}")


def main() -> None:
    witness_path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_WITNESS
    if not witness_path.exists():
        fail(f"missing witness artifact {witness_path}")
    artifact = json.loads(witness_path.read_text(encoding="utf-8"))

    if artifact.get("schema") != WITNESS_SCHEMA:
        fail(f"unexpected witness schema {artifact.get('schema')!r}")

    schema = load_schema()
    commit_fields, commit_widths = load_record_schema(schema, COMMIT_RECORD)
    state_fields, state_widths = load_record_schema(schema, STATE_RECORD)
    commit_ops = load_commit_ops(schema)
    authority_fields = authority_projection_fields(state_fields)

    require_schema_block("commit", artifact.get("commit_schema"), commit_fields, commit_widths)
    require_schema_block("state", artifact.get("state_schema"), state_fields, state_widths)

    records = artifact.get("records")
    if not isinstance(records, list):
        fail("witness artifact has no records list")
    if artifact.get("commit_count") != len(records):
        fail(f"commit_count {artifact.get('commit_count')!r} does not match {len(records)} records")
    if sha256_json(records) != artifact.get("records_sha256"):
        fail("records hash does not recompute; witness artifact is not internally consistent")

    op_values = set(commit_ops.values())
    for idx, record in enumerate(records):
        if record.get("index") != idx:
            fail(f"record {idx} has out-of-order index {record.get('index')!r}")
        commit = record.get("commit")
        pre_state = record.get("pre_state")
        post_state = record.get("post_state")
        if not isinstance(commit, dict) or not isinstance(pre_state, dict) or not isinstance(post_state, dict):
            fail(f"record {idx} is missing commit/pre_state/post_state projections")

        if commit.get("op") != record.get("op") or commit.get("status") != record.get("status"):
            fail(f"record {idx} top-level op/status drifted from commit projection")
        if pre_state.get("pc") is not None or post_state.get("pc") is not None:
            fail(f"record {idx} state projection unexpectedly carries pipeline pc field")
        if commit.get("op") not in op_values:
            fail(f"record {idx} has unknown commit op {commit.get('op')!r}")

        check_packed("commit", idx, record.get("commit_bits"), commit, commit_fields, commit_widths)
        check_packed("pre_state", idx, record.get("pre_state_bits"), pre_state, state_fields, state_widths)
        check_packed("post_state", idx, record.get("post_state_bits"), post_state, state_fields, state_widths)

        check_top_m1_optional_authority_slots(pre_state, idx, "pre")
        check_top_m1_optional_authority_slots(post_state, idx, "post")
        check_top_m1_refinement_step(idx, commit, pre_state, post_state, commit_ops, authority_fields)

    print(f"rtl top-level M1 witness ok ({len(records)} records, {witness_path.name})")


if __name__ == "__main__":
    main()

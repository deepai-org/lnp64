#!/usr/bin/env python3
"""Validate the shared RTL schema manifest against lnp64_pkg.sv and traces."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(os.environ.get("LNP64_SCHEMA_ROOT", str(Path(__file__).resolve().parents[1])))
SCHEMA = Path(os.environ.get("LNP64_SHARED_SCHEMA", str(ROOT / "rtl/schema/lnp64_shared_schema.json")))


def fail(message: str) -> None:
    raise SystemExit(f"rtl shared schema check failed: {message}")


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def read_text(path: Path) -> str:
    if not path.exists():
        fail(f"missing file {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def compact(value: str) -> str:
    return re.sub(r"\s+", "", value.strip())


def bit_width(range_text: str | None) -> int:
    if range_text is None:
        return 1
    match = re.fullmatch(r"\s*(\d+)\s*:\s*(\d+)\s*", range_text)
    require(match is not None, f"unsupported packed range [{range_text}]")
    high = int(match.group(1))
    low = int(match.group(2))
    require(high >= low, f"descending packed range expected, got [{range_text}]")
    return high - low + 1


def parse_parameters(pkg_text: str) -> tuple[dict[str, str], dict[str, str]]:
    simple: dict[str, str] = {}
    rollups: dict[str, str] = {}
    pattern = re.compile(
        r"localparam\s+(?:int|logic(?:\s+\[[^\]]+\])?)\s+(?P<name>\w+)\s*=\s*(?P<value>.*?);",
        re.S,
    )
    for match in pattern.finditer(pkg_text):
        name = match.group("name")
        value = match.group("value")
        if "|" in value or "\n" in value.strip():
            rollups[name] = value
        else:
            simple[name] = compact(value)
    return simple, rollups


def parse_enums(pkg_text: str) -> dict[str, list[str]]:
    enums: dict[str, list[str]] = {}
    pattern = re.compile(
        r"typedef\s+enum\s+logic\s+\[[^\]]+\]\s*\{(?P<body>.*?)\}\s+(?P<name>\w+)\s*;",
        re.S,
    )
    for match in pattern.finditer(pkg_text):
        entries: list[str] = []
        for item in match.group("body").split(","):
            item = item.strip()
            if not item:
                continue
            entry_match = re.fullmatch(r"(?P<name>\w+)\s*=\s*(?P<value>.+)", item)
            require(entry_match is not None, f"could not parse enum entry in {match.group('name')}: {item}")
            entries.append(f"{entry_match.group('name')}={compact(entry_match.group('value'))}")
        enums[match.group("name")] = entries
    return enums


def parse_records(pkg_text: str) -> dict[str, list[str]]:
    records: dict[str, list[str]] = {}
    pattern = re.compile(r"typedef\s+struct\s+packed\s*\{(?P<body>.*?)\}\s+(?P<name>\w+)\s*;", re.S)
    field_pattern = re.compile(r"\blogic(?:\s+\[(?P<range>[^\]]+)\])?\s+(?P<name>\w+)\s*;")
    for match in pattern.finditer(pkg_text):
        fields = [
            f"{field.group('name')}:{bit_width(field.group('range'))}"
            for field in field_pattern.finditer(match.group("body"))
        ]
        records[match.group("name")] = fields
    return records


def parse_parameter_entry(entry: str) -> tuple[str, str]:
    match = re.fullmatch(r"(?P<name>\w+)=(?P<value>.+)", entry)
    require(match is not None, f"invalid parameter schema entry: {entry}")
    return match.group("name"), compact(match.group("value"))


def parse_record_entry(entry: str) -> tuple[str, int]:
    match = re.fullmatch(r"(?P<name>\w+):(?P<width>\d+)", entry)
    require(match is not None, f"invalid record schema entry: {entry}")
    width = int(match.group("width"))
    require(width > 0, f"record field width must be positive: {entry}")
    return match.group("name"), width


def require_exact_map(actual: dict[str, list[str]], expected: dict[str, list[str]], label: str) -> None:
    require(set(actual) == set(expected), f"{label} names drifted: actual={sorted(actual)} expected={sorted(expected)}")
    for name, expected_items in expected.items():
        require(actual[name] == expected_items, f"{label} {name} drifted: actual={actual[name]} expected={expected_items}")


def require_string(value: object, label: str) -> str:
    require(isinstance(value, str) and value, f"{label} must be a non-empty string")
    return value


def require_mapping_keys(entries: object, expected_keys: list[str], required_fields: list[str], label: str) -> list[dict[str, str]]:
    require(isinstance(entries, list) and entries, f"{label} must be a non-empty list")
    parsed: list[dict[str, str]] = []
    for entry in entries:
        require(isinstance(entry, dict), f"{label} entry must be an object: {entry!r}")
        parsed_entry = {
            field: require_string(entry.get(field), f"{label}.{field}")
            for field in required_fields
        }
        parsed.append(parsed_entry)
    actual_keys = [entry["key"] for entry in parsed]
    require(actual_keys == expected_keys, f"{label} keys drifted: actual={actual_keys} expected={expected_keys}")
    return parsed


def render_sv_struct_from_schema(record_name: str, fields: list[str]) -> str:
    lines = ["typedef struct packed {"]
    for entry in fields:
        field_name, width = parse_record_entry(entry)
        if width == 1:
            lines.append(f"    logic {field_name};")
        else:
            lines.append(f"    logic [{width - 1}:0] {field_name};")
    lines.append(f"}} {record_name};")
    return "\n".join(lines)


def require_m1_generated_structs(
    actual_records: dict[str, list[str]],
    schema: dict,
    m1_contract: dict,
) -> None:
    """Treat the shared schema as the M1 SystemVerilog struct source of truth."""
    schema_records = schema.get("records", {})
    require(isinstance(schema_records, dict), "schema records must be an object")
    for contract_key, label in (
        ("record", "commit"),
        ("state_record", "state projection"),
    ):
        record_name = require_string(m1_contract.get(contract_key), f"M1 {label} record")
        require(record_name in schema_records, f"M1 {label} record is absent from schema records")
        require(record_name in actual_records, f"M1 {label} record is absent from parsed package records")
        expected_sv = render_sv_struct_from_schema(record_name, schema_records[record_name])
        actual_sv = render_sv_struct_from_schema(record_name, actual_records[record_name])
        require(
            actual_sv == expected_sv,
            f"M1 schema-owned generated SV struct {record_name} drifted from shared schema",
        )


def main() -> None:
    schema = json.loads(read_text(SCHEMA))
    require(schema.get("schema") == "lnp64_shared_schema_v1", "unexpected schema id")
    require(schema.get("stage") == "checked_manifest", "shared schema must be a checked manifest")

    package_path = ROOT / schema.get("package", "")
    trace_path = ROOT / schema.get("trace_manifest", "")
    pkg_text = read_text(package_path)
    trace_manifest = json.loads(read_text(trace_path))

    actual_parameters, actual_rollups = parse_parameters(pkg_text)
    expected_parameters = dict(parse_parameter_entry(entry) for entry in schema.get("parameters", []))
    require(
        set(actual_parameters) == set(expected_parameters),
        f"parameter names drifted: actual={sorted(actual_parameters)} expected={sorted(expected_parameters)}",
    )
    for name, expected_value in expected_parameters.items():
        require(actual_parameters[name] == expected_value, f"parameter {name} drifted: actual={actual_parameters[name]} expected={expected_value}")

    expected_rollups = schema.get("rollups", {})
    require(set(actual_rollups) == set(expected_rollups), "feature rollup names drifted")
    for name, expected_terms in expected_rollups.items():
        body_terms = re.findall(r"\bLNP64_FEATURE_[A-Z0-9_]+\b", actual_rollups[name])
        require(body_terms == expected_terms, f"rollup {name} drifted: actual={body_terms} expected={expected_terms}")

    require_exact_map(parse_enums(pkg_text), schema.get("enums", {}), "enum")
    actual_records = parse_records(pkg_text)
    require_exact_map(actual_records, schema.get("records", {}), "record")

    families = schema.get("record_families", {})
    require(isinstance(families, dict) and families, "record_families must be non-empty")
    family_records = [record for records in families.values() for record in records]
    require(sorted(family_records) == sorted(schema["records"]), "record_families must cover every record exactly once")
    require(len(family_records) == len(set(family_records)), "record_families contains duplicate records")

    trace_contract = schema.get("trace_contract", {})
    require(trace_contract.get("stage") == "string_trace_scaffold", "current trace stage must be honest")
    require(trace_contract.get("typed_record") in schema["records"], "trace typed_record must name a package record")
    actual_categories = list(trace_manifest.get("trace_comparison_contract", {}))
    require(actual_categories == trace_contract.get("categories"), "trace comparison categories drifted from shared schema")

    typed_contract = schema.get("typed_trace_contract", {})
    require(isinstance(typed_contract, dict) and typed_contract, "typed_trace_contract must be present")
    require(
        typed_contract.get("stage") == "s0_runtime_typed_trace_scaffold",
        "typed trace stage must be the honest S0 runtime scaffold",
    )
    typed_gate = typed_contract.get("gate")
    typed_source = typed_contract.get("source")
    require(isinstance(typed_gate, str) and (ROOT / typed_gate).exists(), "typed trace gate must exist")
    require(isinstance(typed_source, str) and (ROOT / typed_source).exists(), "typed trace source must exist")
    required_records = typed_contract.get("required_records")
    require(isinstance(required_records, list) and required_records, "typed trace required_records must be non-empty")
    missing_records = sorted(set(required_records) - set(schema["records"]))
    require(not missing_records, f"typed trace records are absent from schema: {missing_records}")

    m1_contract = schema.get("m1_typed_commit_contract", {})
    require(isinstance(m1_contract, dict) and m1_contract, "m1_typed_commit_contract must be present")
    require_m1_generated_structs(actual_records, schema, m1_contract)
    require(
        m1_contract.get("stage") == "m1_typed_cap_commit_transition_mirror",
        "M1 typed commit stage must identify the transition mirror",
    )
    m1_gate = m1_contract.get("gate")
    m1_source = m1_contract.get("source")
    m1_record = m1_contract.get("record")
    m1_state_record = m1_contract.get("state_record")
    m1_op_enum = m1_contract.get("op_enum")
    require(isinstance(m1_gate, str) and (ROOT / m1_gate).exists(), "M1 typed commit gate must exist")
    require(isinstance(m1_source, str) and (ROOT / m1_source).exists(), "M1 typed commit source must exist")
    require(m1_record in schema["records"], "M1 typed commit record must name a package record")
    require(m1_state_record in schema["records"], "M1 state projection record must name a package record")
    require(m1_op_enum in schema["enums"], "M1 typed commit op_enum must name a package enum")
    require(m1_contract.get("record_name") == "m1_cap_commit", "M1 typed commit record_name drifted")
    require(
        m1_contract.get("state_record_name") == "m1_state_projection",
        "M1 state projection record_name drifted",
    )
    op_mappings = require_mapping_keys(
        m1_contract.get("op_mappings"),
        [
            "cap_dup",
            "cap_send",
            "cap_recv",
            "cap_revoke",
            "reject_stale",
            "push",
            "pull",
            "reject_full",
            "cap_dup_denied",
            "object_create",
        ],
        ["key", "sv", "lean_op", "lean_commit_op", "lean_transition"],
        "M1 op_mappings",
    )
    m1_op_enum_names = {entry.split("=", 1)[0] for entry in schema["enums"][m1_op_enum]}
    missing_sv_ops = sorted({entry["sv"] for entry in op_mappings} - m1_op_enum_names)
    require(not missing_sv_ops, f"M1 op_mappings reference absent SV enum entries: {missing_sv_ops}")
    status_mappings = require_mapping_keys(
        m1_contract.get("status_mappings"),
        ["ok", "eperm", "eagain", "erevoked"],
        ["key", "sv_errno", "lean_status"],
        "M1 status_mappings",
    )
    errno_enum_names = {entry.split("=", 1)[0] for entry in schema["enums"]["lnp64_errno_e"]}
    missing_statuses = sorted({entry["sv_errno"] for entry in status_mappings} - errno_enum_names)
    require(not missing_statuses, f"M1 status_mappings reference absent errno entries: {missing_statuses}")

    print("rtl shared schema ok")


if __name__ == "__main__":
    main()

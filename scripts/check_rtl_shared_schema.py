#!/usr/bin/env python3
"""Validate the shared RTL schema manifest against lnp64_pkg.sv and traces."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(os.environ.get("LNP64_SCHEMA_ROOT", str(Path(__file__).resolve().parents[1])))
SCHEMA = Path(os.environ.get("LNP64_SHARED_SCHEMA", str(ROOT / "rtl/schema/lnp64_shared_schema.json")))
LEAN_M1_MODEL = ROOT / "formal/M1TransitionInvariantModel.lean"


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


def parse_enum_entry_value(entry: str) -> tuple[str, int]:
    match = re.fullmatch(r"(?P<name>\w+)=\d+'d(?P<value>\d+)", compact(entry))
    require(match is not None, f"unsupported enum schema entry: {entry}")
    return match.group("name"), int(match.group("value"))


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


def render_lean_packed_schema(schema_name: str, fields: list[str]) -> str:
    lines = [f"def {schema_name} : List (String × Nat) := ["]
    for index, entry in enumerate(fields):
        field_name, width = parse_record_entry(entry)
        comma = "," if index + 1 < len(fields) else ""
        lines.append(f'  ("{field_name}", {width}){comma}')
    lines.append("]")
    return "\n".join(lines)


def render_lean_width_theorem(theorem_name: str, schema_name: str, fields: list[str]) -> str:
    width = sum(parse_record_entry(entry)[1] for entry in fields)
    return (
        f"theorem {theorem_name} :\n"
        f"    packedSchemaWidth {schema_name} = {width} := by\n"
        "  rfl"
    )


def render_lean_packed_layout(layout_name: str, fields: list[str]) -> str:
    total_width = sum(parse_record_entry(entry)[1] for entry in fields)
    cursor = total_width
    lines = [f"def {layout_name} : List PackedFieldLayout := ["]
    for index, entry in enumerate(fields):
        field_name, width = parse_record_entry(entry)
        lsb = cursor - width
        msb = cursor - 1
        cursor = lsb
        comma = "," if index + 1 < len(fields) else ""
        lines.append(
            f'  {{ name := "{field_name}", width := {width}, lsb := {lsb}, msb := {msb} }}{comma}'
        )
    lines.append("]")
    return "\n".join(lines)


def render_lean_layout_theorem(theorem_name: str, schema_name: str, layout_name: str) -> str:
    return (
        f"theorem {theorem_name} :\n"
        f"    packedSchemaLayout {schema_name} =\n"
        f"      {layout_name} := by\n"
        "  rfl"
    )


def render_lean_layout_bool_theorem(
    theorem_name: str,
    predicate_name: str,
    schema_name: str,
    layout_name: str,
) -> str:
    return (
        f"theorem {theorem_name} :\n"
        f"    {predicate_name}\n"
        f"      (packedSchemaWidth {schema_name})\n"
        f"      {layout_name} = true := by\n"
        "  rfl"
    )


def render_lean_commit_op_decoder(op_mappings: list[dict[str, str]], enum_values: dict[str, int]) -> str:
    lines = ["def commitOpFromPackedValue : Nat -> Option CommitOp"]
    for entry in op_mappings:
        sv_name = entry["sv"]
        require(sv_name in enum_values, f"M1 op decoder references absent enum {sv_name}")
        lines.append(f"  | {enum_values[sv_name]} => some CommitOp.{entry['lean_commit_op']}")
    lines.append("  | _ => none")
    return "\n".join(lines)


def render_lean_commit_status_decoder(
    status_mappings: list[dict[str, str]],
    enum_values: dict[str, int],
) -> str:
    lines = ["def commitStatusFromPackedValue : Nat -> Option CommitStatus"]
    for entry in status_mappings:
        sv_name = entry["sv_errno"]
        require(sv_name in enum_values, f"M1 status decoder references absent enum {sv_name}")
        lines.append(f"  | {enum_values[sv_name]} => some CommitStatus.{entry['lean_status']}")
    lines.append("  | _ => none")
    return "\n".join(lines)


def parse_lean_packed_schema(lean_text: str, schema_name: str) -> list[str]:
    pattern = re.compile(
        rf"def\s+{re.escape(schema_name)}\s*:\s*List\s*\(String\s*×\s*Nat\)\s*:=\s*\[(?P<body>.*?)\]",
        re.S,
    )
    match = pattern.search(lean_text)
    require(match is not None, f"missing Lean packed schema {schema_name}")
    entries = re.findall(r'\(\s*"(?P<name>[^"]+)"\s*,\s*(?P<width>\d+)\s*\)', match.group("body"))
    require(entries, f"Lean packed schema {schema_name} has no fields")
    return [f"{name}:{int(width)}" for name, width in entries]


def parse_lean_packed_layout(lean_text: str, layout_name: str) -> list[str]:
    pattern = re.compile(
        rf"def\s+{re.escape(layout_name)}\s*:\s*List\s+PackedFieldLayout\s*:=\s*\[(?P<body>.*?)\]",
        re.S,
    )
    match = pattern.search(lean_text)
    require(match is not None, f"missing Lean packed layout {layout_name}")
    entries = re.findall(
        r'\{\s*name\s*:=\s*"(?P<name>[^"]+)"\s*,\s*'
        r"width\s*:=\s*(?P<width>\d+)\s*,\s*"
        r"lsb\s*:=\s*(?P<lsb>\d+)\s*,\s*"
        r"msb\s*:=\s*(?P<msb>\d+)\s*\}",
        match.group("body"),
    )
    require(entries, f"Lean packed layout {layout_name} has no fields")
    return [
        f"{name}:{int(width)}:{int(lsb)}:{int(msb)}"
        for name, width, lsb, msb in entries
    ]


def expected_packed_layout_entries(fields: list[str]) -> list[str]:
    total_width = sum(parse_record_entry(entry)[1] for entry in fields)
    cursor = total_width
    entries: list[str] = []
    for entry in fields:
        field_name, width = parse_record_entry(entry)
        lsb = cursor - width
        msb = cursor - 1
        entries.append(f"{field_name}:{width}:{lsb}:{msb}")
        cursor = lsb
    return entries


def parse_lean_width_theorem(lean_text: str, theorem_name: str, schema_name: str) -> int:
    pattern = re.compile(
        rf"theorem\s+{re.escape(theorem_name)}\s*:\s*"
        rf"packedSchemaWidth\s+{re.escape(schema_name)}\s*=\s*(?P<width>\d+)\s*:=\s*by",
        re.S,
    )
    match = pattern.search(lean_text)
    require(match is not None, f"missing Lean packed-schema width theorem {theorem_name}")
    return int(match.group("width"))


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


def require_m1_generated_lean_packed_schemas(schema: dict, m1_contract: dict, lean_text: str) -> None:
    """Treat the shared schema as the M1 Lean packed-schema source of truth."""
    schema_records = schema.get("records", {})
    require(isinstance(schema_records, dict), "schema records must be an object")
    pairs = (
        (
            "record",
            "rtlM1CommitPackedSchema",
            "rtlM1CommitPackedSchema_width",
            "rtlM1CommitPackedLayout",
            "rtlM1CommitPackedLayout_from_schema",
            "rtlM1CommitPackedLayout_within_schema_width",
            "rtlM1CommitPackedLayout_covers_schema_width",
        ),
        (
            "state_record",
            "rtlM1StateProjectionPackedSchema",
            "rtlM1StateProjectionPackedSchema_width",
            "rtlM1StateProjectionPackedLayout",
            "rtlM1StateProjectionPackedLayout_from_schema",
            "rtlM1StateProjectionPackedLayout_within_schema_width",
            "rtlM1StateProjectionPackedLayout_covers_schema_width",
        ),
    )
    for (
        contract_key,
        schema_name,
        theorem_name,
        layout_name,
        layout_theorem_name,
        layout_within_theorem_name,
        layout_covers_theorem_name,
    ) in pairs:
        record_name = require_string(m1_contract.get(contract_key), f"M1 Lean {schema_name} record")
        fields = schema_records.get(record_name)
        require(isinstance(fields, list) and fields, f"M1 Lean {schema_name} record is absent from schema")
        actual_fields = parse_lean_packed_schema(lean_text, schema_name)
        require(
            actual_fields == fields,
            f"M1 schema-owned generated Lean packed schema {schema_name} drifted from shared schema",
        )
        expected_schema = render_lean_packed_schema(schema_name, fields)
        require(
            expected_schema in lean_text,
            f"M1 generated Lean packed schema text for {schema_name} drifted from shared schema",
        )
        actual_width = parse_lean_width_theorem(lean_text, theorem_name, schema_name)
        expected_width = sum(parse_record_entry(entry)[1] for entry in fields)
        require(
            actual_width == expected_width,
            f"M1 schema-owned Lean packed-schema width {theorem_name} drifted from shared schema",
        )
        expected_theorem = render_lean_width_theorem(theorem_name, schema_name, fields)
        require(
            expected_theorem in lean_text,
            f"M1 generated Lean packed-schema width theorem {theorem_name} drifted from shared schema",
        )
        actual_layout = parse_lean_packed_layout(lean_text, layout_name)
        expected_layout = expected_packed_layout_entries(fields)
        require(
            actual_layout == expected_layout,
            f"M1 schema-owned Lean packed layout {layout_name} drifted from shared schema",
        )
        expected_layout_text = render_lean_packed_layout(layout_name, fields)
        require(
            expected_layout_text in lean_text,
            f"M1 generated Lean packed layout text for {layout_name} drifted from shared schema",
        )
        expected_layout_theorem = render_lean_layout_theorem(
            layout_theorem_name,
            schema_name,
            layout_name,
        )
        require(
            expected_layout_theorem in lean_text,
            f"M1 generated Lean packed-layout theorem {layout_theorem_name} drifted from shared schema",
        )
        expected_within_theorem = render_lean_layout_bool_theorem(
            layout_within_theorem_name,
            "packedLayoutWithinWidth",
            schema_name,
            layout_name,
        )
        require(
            expected_within_theorem in lean_text,
            f"M1 generated Lean packed-layout bounds theorem {layout_within_theorem_name} drifted from shared schema",
        )
        expected_covers_theorem = render_lean_layout_bool_theorem(
            layout_covers_theorem_name,
            "packedLayoutCoversWidth",
            schema_name,
            layout_name,
        )
        require(
            expected_covers_theorem in lean_text,
            f"M1 generated Lean packed-layout coverage theorem {layout_covers_theorem_name} drifted from shared schema",
        )


def require_m1_packed_bit_refinement_contract(schema: dict, m1_contract: dict, lean_text: str) -> None:
    """Require the M1 Lean model to relate emitted packed bits to projections."""
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
    status_mappings = require_mapping_keys(
        m1_contract.get("status_mappings"),
        ["ok", "eperm", "eagain", "erevoked"],
        ["key", "sv_errno", "lean_status"],
        "M1 status_mappings",
    )
    enums = schema.get("enums", {})
    require(isinstance(enums, dict), "schema enums must be an object")
    op_enum = require_string(m1_contract.get("op_enum"), "M1 op enum")
    require(op_enum in enums, "M1 op enum is absent from schema")
    require("lnp64_errno_e" in enums, "errno enum is absent from schema")
    op_enum_values = dict(parse_enum_entry_value(entry) for entry in enums[op_enum])
    status_enum_values = dict(parse_enum_entry_value(entry) for entry in enums["lnp64_errno_e"])
    expected_op_decoder = render_lean_commit_op_decoder(op_mappings, op_enum_values)
    require(
        expected_op_decoder in lean_text,
        "M1 generated Lean packed-bit op decoder drifted from shared schema",
    )
    expected_status_decoder = render_lean_commit_status_decoder(status_mappings, status_enum_values)
    require(
        expected_status_decoder in lean_text,
        "M1 generated Lean packed-bit status decoder drifted from shared schema",
    )
    required_artifacts = [
        "def packedBitSlice",
        "def packedLayoutFieldValue",
        "def rightsFromPackedMask",
        "def modeledRightsMaskLimit",
        "def packedRightsFieldModeled",
        "def packedRightsFromLayoutMatches",
        "theorem rightsFromPackedMask_allRights",
        "theorem rightsFromPackedMask_pullOnly",
        "theorem rightsFromPackedMask_noRights",
        "structure RtlM1CommitProjectionFromPackedBits",
        "structure RtlM1StateProjectionFromPackedBits",
        "theorem rtl_m1_commit_projection_from_packed_bits_within_schema_width",
        "theorem rtl_m1_state_projection_from_packed_bits_within_schema_width",
        "theorem rtl_m1_commit_projection_from_packed_bits_rights_modeled",
        "theorem rtl_m1_state_projection_from_packed_bits_rights_modeled",
        "structure RtlM1PackedRefinementStep",
        "theorem rtl_m1_packed_refinement_step_refines_lean_step",
        "theorem rtl_m1_packed_refinement_step_status_matches_op",
        "theorem rtl_m1_packed_refinement_step_preserves_sg_auth_invariant",
    ]
    for artifact in required_artifacts:
        require(artifact in lean_text, f"M1 packed-bit refinement artifact missing: {artifact}")


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
    lean_m1_model = read_text(LEAN_M1_MODEL)
    require_m1_generated_lean_packed_schemas(schema, m1_contract, lean_m1_model)
    require_m1_packed_bit_refinement_contract(schema, m1_contract, lean_m1_model)
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

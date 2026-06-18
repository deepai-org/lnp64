#!/usr/bin/env python3
"""Self-test M1 typed commit checker Lean packed-schema failure modes."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m1_typed_commit_trace.py"
SCHEMA = ROOT / "rtl/schema/lnp64_shared_schema.json"
LEAN_M1_MODEL = ROOT / "formal/M1TransitionInvariantModel.lean"
RTL_M1_ENGINE = ROOT / "rtl/engines/lnp64_m1_pingpong.sv"
RTL_M1_TB = ROOT / "rtl/sim/lnp64_m1_tb.sv"
RTL_M1_ASSERTIONS = ROOT / "formal/rtl_assertions/lnp64_m1_assertions.sv"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m1_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M1 checker module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def schema_specs(checker) -> tuple[tuple[tuple[str, int], ...], tuple[tuple[str, int], ...]]:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    contract = schema["m1_typed_commit_contract"]
    commit_fields = tuple(
        checker.parse_schema_field(entry)
        for entry in schema["records"][contract["record"]]
    )
    state_fields = tuple(
        checker.parse_schema_field(entry)
        for entry in schema["records"][contract["state_record"]]
    )
    return commit_fields, state_fields


def expect_failure(expected: str, action) -> None:
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        try:
            action()
        except SystemExit as exc:
            require(exc.code != 0, "checker failure unexpectedly used success exit code")
        else:
            raise SystemExit("expected checker failure")
    output = stderr.getvalue()
    require(expected in output, f"checker failure did not include {expected!r}: {output}")


def replace_once(source: str, old: str, new: str) -> str:
    require(old in source, f"Lean source did not contain {old!r}")
    return source.replace(old, new, 1)


def main() -> None:
    checker = load_checker()
    commit_fields, state_fields = schema_specs(checker)
    commit_field_names = tuple(name for name, _width in commit_fields)
    state_field_names = tuple(name for name, _width in state_fields)
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    m1_contract = schema["m1_typed_commit_contract"]
    lean_source = LEAN_M1_MODEL.read_text(encoding="utf-8")
    engine_source = RTL_M1_ENGINE.read_text(encoding="utf-8")
    tb_source = RTL_M1_TB.read_text(encoding="utf-8")
    assertion_source = RTL_M1_ASSERTIONS.read_text(encoding="utf-8")

    checker.check_lean_packed_schema_contract(lean_source, commit_fields, state_fields)
    checker.check_rtl_state_projection_boundary_sources(
        engine_source,
        tb_source,
        assertion_source,
        commit_field_names,
        state_field_names,
    )

    checker.check_lean_typed_commit_mapping(
        checker.load_m1_op_mappings(m1_contract),
        checker.load_m1_status_mappings(m1_contract),
    )

    bad_op_key_contract = json.loads(json.dumps(m1_contract))
    bad_op_key_contract["op_mappings"][0]["key"] = "cap_dup_renamed"
    expect_failure(
        "op mapping keys drifted",
        lambda: checker.load_m1_op_mappings(bad_op_key_contract),
    )

    bad_lean_transition_contract = json.loads(json.dumps(m1_contract))
    bad_lean_transition_contract["op_mappings"][0]["lean_transition"] = "missingLeanTransition"
    expect_failure(
        "missing Lean TypedCommitTransition constructors",
        lambda: checker.check_lean_typed_commit_mapping(
            checker.load_m1_op_mappings(bad_lean_transition_contract),
            checker.load_m1_status_mappings(bad_lean_transition_contract),
        ),
    )

    expect_failure(
        "every commit trace field from typed_commit",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            tb_source,
            assertion_source,
            commit_field_names + ("schema_added_commit_field",),
            state_field_names,
        ),
    )

    expect_failure(
        "every state trace field from typed_state_projection",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            tb_source,
            assertion_source,
            commit_field_names,
            state_field_names + ("schema_added_state_field",),
        ),
    )

    missing_commit_field = replace_once(
        tb_source,
        "typed_commit.object_id,",
        "typed_commit.object_gen,",
    )
    expect_failure(
        "every commit trace field from typed_commit",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            missing_commit_field,
            assertion_source,
            commit_field_names,
            state_field_names,
        ),
    )

    wrong_commit_bits_source = replace_once(
        tb_source,
        "typed_commit\n            );\n            $display(\n                \"TTRACE_M1_STATE",
        "typed_state_projection\n            );\n            $display(\n                \"TTRACE_M1_STATE",
    )
    expect_failure(
        "packed commit bits from typed_commit",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            wrong_commit_bits_source,
            assertion_source,
            commit_field_names,
            state_field_names,
        ),
    )

    missing_commit_schema = replace_once(
        lean_source,
        "def rtlM1CommitPackedSchema :",
        "def rtlM1CommitPackedSchemaMissing :",
    )
    expect_failure(
        "missing packed schema rtlM1CommitPackedSchema",
        lambda: checker.check_lean_packed_schema_contract(
            missing_commit_schema,
            commit_fields,
            state_fields,
        ),
    )

    wrong_commit_width = replace_once(
        lean_source,
        "packedSchemaWidth rtlM1CommitPackedSchema = 281",
        "packedSchemaWidth rtlM1CommitPackedSchema = 280",
    )
    expect_failure(
        "rtlM1CommitPackedSchema_width drifted",
        lambda: checker.check_lean_packed_schema_contract(
            wrong_commit_width,
            commit_fields,
            state_fields,
        ),
    )

    wrong_state_width = replace_once(
        lean_source,
        "packedSchemaWidth rtlM1StateProjectionPackedSchema = 902",
        "packedSchemaWidth rtlM1StateProjectionPackedSchema = 901",
    )
    expect_failure(
        "rtlM1StateProjectionPackedSchema_width drifted",
        lambda: checker.check_lean_packed_schema_contract(
            wrong_state_width,
            commit_fields,
            state_fields,
        ),
    )

    wrong_field_width = replace_once(
        lean_source,
        '("rights_mask", 64)',
        '("rights_mask", 63)',
    )
    expect_failure(
        "rtlM1CommitPackedSchema drifted",
        lambda: checker.check_lean_packed_schema_contract(
            wrong_field_width,
            commit_fields,
            state_fields,
        ),
    )

    print("rtl m1 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

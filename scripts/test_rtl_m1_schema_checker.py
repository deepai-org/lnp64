#!/usr/bin/env python3
"""Self-test M1 schema-owned SystemVerilog struct checker failure modes."""

from __future__ import annotations

import copy
import contextlib
import importlib.util
import io
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_shared_schema.py"
SCHEMA = ROOT / "rtl/schema/lnp64_shared_schema.json"
PKG = ROOT / "rtl/include/lnp64_pkg.sv"
LEAN_M1_MODEL = ROOT / "formal/M1TransitionInvariantModel.lean"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_shared_schema", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load shared schema checker")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def expect_failure(expected: str, action) -> None:
    stderr = io.StringIO()
    failure_text = ""
    with contextlib.redirect_stderr(stderr):
        try:
            action()
        except SystemExit as exc:
            require(exc.code != 0, "checker failure unexpectedly used success exit code")
            failure_text = str(exc)
        else:
            raise SystemExit("expected checker failure")
    output = stderr.getvalue() + failure_text
    require(expected in output, f"checker failure did not include {expected!r}: {output}")


def replace_once(text: str, old: str, new: str) -> str:
    require(old in text, f"test fixture did not contain {old!r}")
    return text.replace(old, new, 1)


def main() -> None:
    checker = load_checker()
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    m1_contract = schema["m1_typed_commit_contract"]
    actual_records = checker.parse_records(PKG.read_text(encoding="utf-8"))
    lean_source = LEAN_M1_MODEL.read_text(encoding="utf-8")

    valid = subprocess.run(
        [sys.executable, str(CHECKER)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    valid_output = (valid.stdout or "") + (valid.stderr or "")
    require(valid.returncode == 0, f"current shared schema failed: {valid_output}")
    require("rtl shared schema ok" in valid_output, "current shared schema did not print success")

    checker.require_m1_generated_structs(actual_records, schema, m1_contract)
    checker.require_m1_generated_lean_packed_schemas(schema, m1_contract, lean_source)
    checker.require_m1_packed_bit_refinement_contract(schema, m1_contract, lean_source)

    package_width_drift = copy.deepcopy(actual_records)
    package_width_drift["lnp64_m1_cap_commit_t"] = [
        "rights_mask:63" if field == "rights_mask:64" else field
        for field in package_width_drift["lnp64_m1_cap_commit_t"]
    ]
    expect_failure(
        "M1 schema-owned generated SV struct lnp64_m1_cap_commit_t drifted",
        lambda: checker.require_m1_generated_structs(
            package_width_drift,
            schema,
            m1_contract,
        ),
    )

    schema_field_drift = copy.deepcopy(schema)
    schema_field_drift["records"]["lnp64_m1_state_projection_t"] = [
        "transfer_valid:2" if field == "transfer_valid:1" else field
        for field in schema_field_drift["records"]["lnp64_m1_state_projection_t"]
    ]
    expect_failure(
        "M1 schema-owned generated SV struct lnp64_m1_state_projection_t drifted",
        lambda: checker.require_m1_generated_structs(
            actual_records,
            schema_field_drift,
            schema_field_drift["m1_typed_commit_contract"],
        ),
    )

    lean_field_width_drift = replace_once(
        lean_source,
        '("rights_mask", 64)',
        '("rights_mask", 63)',
    )
    expect_failure(
        "M1 schema-owned generated Lean packed schema rtlM1CommitPackedSchema drifted",
        lambda: checker.require_m1_generated_lean_packed_schemas(
            schema,
            m1_contract,
            lean_field_width_drift,
        ),
    )

    lean_width_theorem_drift = replace_once(
        lean_source,
        "packedSchemaWidth rtlM1StateProjectionPackedSchema = 902",
        "packedSchemaWidth rtlM1StateProjectionPackedSchema = 901",
    )
    expect_failure(
        "M1 schema-owned Lean packed-schema width rtlM1StateProjectionPackedSchema_width drifted",
        lambda: checker.require_m1_generated_lean_packed_schemas(
            schema,
            m1_contract,
            lean_width_theorem_drift,
        ),
    )

    lean_layout_drift = replace_once(
        lean_source,
        '{ name := "rights_mask", width := 64, lsb := 49, msb := 112 }',
        '{ name := "rights_mask", width := 64, lsb := 48, msb := 111 }',
    )
    expect_failure(
        "M1 schema-owned Lean packed layout rtlM1CommitPackedLayout drifted",
        lambda: checker.require_m1_generated_lean_packed_schemas(
            schema,
            m1_contract,
            lean_layout_drift,
        ),
    )

    lean_layout_bounds_theorem_drift = replace_once(
        lean_source,
        "packedLayoutWithinWidth\n      (packedSchemaWidth rtlM1CommitPackedSchema)",
        "packedLayoutWithinWidth\n      (packedSchemaWidth rtlM1StateProjectionPackedSchema)",
    )
    expect_failure(
        "M1 generated Lean packed-layout bounds theorem rtlM1CommitPackedLayout_within_schema_width drifted",
        lambda: checker.require_m1_generated_lean_packed_schemas(
            schema,
            m1_contract,
            lean_layout_bounds_theorem_drift,
        ),
    )

    lean_layout_coverage_theorem_drift = replace_once(
        lean_source,
        "packedLayoutCoversWidth\n      (packedSchemaWidth rtlM1StateProjectionPackedSchema)",
        "packedLayoutCoversWidth\n      (packedSchemaWidth rtlM1CommitPackedSchema)",
    )
    expect_failure(
        "M1 generated Lean packed-layout coverage theorem rtlM1StateProjectionPackedLayout_covers_schema_width drifted",
        lambda: checker.require_m1_generated_lean_packed_schemas(
            schema,
            m1_contract,
            lean_layout_coverage_theorem_drift,
        ),
    )

    lean_op_decoder_drift = replace_once(
        lean_source,
        "| 2 => some CommitOp.capSend",
        "| 2 => some CommitOp.capRecv",
    )
    expect_failure(
        "M1 generated Lean packed-bit op decoder drifted from shared schema",
        lambda: checker.require_m1_packed_bit_refinement_contract(
            schema,
            m1_contract,
            lean_op_decoder_drift,
        ),
    )

    lean_status_decoder_drift = replace_once(
        lean_source,
        "| 122 => some CommitStatus.erevoked",
        "| 122 => some CommitStatus.eagain",
    )
    expect_failure(
        "M1 generated Lean packed-bit status decoder drifted from shared schema",
        lambda: checker.require_m1_packed_bit_refinement_contract(
            schema,
            m1_contract,
            lean_status_decoder_drift,
        ),
    )

    missing_packed_refinement_theorem = replace_once(
        lean_source,
        "theorem rtl_m1_packed_refinement_step_refines_lean_step",
        "theorem rtl_m1_packed_step_no_refinement_artifact",
    )
    expect_failure(
        "M1 packed-bit refinement artifact missing: theorem rtl_m1_packed_refinement_step_refines_lean_step",
        lambda: checker.require_m1_packed_bit_refinement_contract(
            schema,
            m1_contract,
            missing_packed_refinement_theorem,
        ),
    )

    print("rtl m1 schema checker self-test ok")


if __name__ == "__main__":
    main()

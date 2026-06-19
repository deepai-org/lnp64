#!/usr/bin/env python3
"""Check the feature-gated top-level RTL program-test manifest."""

from __future__ import annotations

import json
import os
from pathlib import Path


ROOT = Path(os.environ.get("LNP64_TOP_PROGRAM_ROOT", str(Path(__file__).resolve().parents[1])))
MANIFEST = Path(
    os.environ.get(
        "LNP64_TOP_PROGRAM_MANIFEST",
        str(ROOT / "tests/rtl/top_level_program_manifest.json"),
    )
)
LLVM_BOOTSTRAP = ROOT / "toolchain/lnp64_llvm_bootstrap.manifest"
LINKED_REPLACEMENTS = {
    "tests/rtl/programs/top_return_12.c": (
        "tests/rtl/programs/top_linked_main.c",
        "startup_call_main",
    ),
    "tests/rtl/programs/top_branch_if.c": (
        "tests/rtl/programs/top_linked_loop_branch.c",
        "branch",
    ),
    "tests/rtl/programs/top_loop_sum.c": (
        "tests/rtl/programs/top_linked_loop_branch.c",
        "branch",
    ),
    "tests/rtl/programs/top_factorial_mul.c": (
        "tests/rtl/programs/top_linked_factorial_mul.c",
        "mul",
    ),
    "tests/rtl/programs/top_subtract.c": (
        "tests/rtl/programs/top_linked_loop_branch.c",
        "call_return",
    ),
    "tests/rtl/programs/top_bitwise.c": (
        "tests/rtl/programs/top_linked_bitwise_shift.c",
        "bitwise_alu",
    ),
    "tests/rtl/programs/top_shift.c": (
        "tests/rtl/programs/top_linked_bitwise_shift.c",
        "shift_alu",
    ),
    "tests/rtl/programs/top_udiv_urem.c": (
        "tests/rtl/programs/top_linked_divrem.c",
        "unsigned_division",
    ),
    "tests/rtl/programs/top_signed_division.c": (
        "tests/rtl/programs/top_linked_divrem.c",
        "signed_division",
    ),
    "tests/rtl/programs/top_not.c": (
        "tests/rtl/programs/top_linked_bitwise_shift.c",
        "bitwise_alu",
    ),
    "tests/rtl/programs/top_call_return.c": (
        "tests/rtl/programs/top_linked_loop_branch.c",
        "call_return",
    ),
    "tests/rtl/programs/top_byte_array.c": (
        "tests/rtl/programs/top_linked_byte_array.c",
        "byte_load_store",
    ),
    "tests/rtl/programs/top_heap_byte_lanes.c": (
        "tests/rtl/programs/top_linked_heap_byte_lanes.c",
        "heap",
    ),
    "demos/allocator.c": ("tests/rtl/programs/top_linked_allocator_native.c", "heap"),
    "demos/factorial.c": ("tests/rtl/programs/top_linked_factorial_native.c", "push_pull"),
    "demos/fibonacci.c": ("tests/rtl/programs/top_linked_fibonacci_native.c", "call_return"),
    "demos/hello.c": ("tests/rtl/programs/top_linked_hello_native.c", "push_pull"),
    "demos/json_parser.c": ("tests/rtl/programs/top_linked_json_parser_native.c", "heap"),
    "demos/ping_pong.c": ("tests/rtl/programs/top_linked_clone_join.c", "thread_join"),
    "demos/rot13.c": ("tests/rtl/programs/top_linked_rot13_native.c", "push_pull"),
}


def fail(message: str) -> None:
    raise SystemExit(f"rtl top-level program manifest check failed: {message}")


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def text(path: Path) -> str:
    require(path.exists(), f"missing file {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def llvm_bootstrap_demo_c_sources(manifest_text: str) -> set[str]:
    sources: set[str] = set()
    for raw_line in manifest_text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("|")
        require(len(fields) >= 2, f"invalid LLVM bootstrap manifest row: {line}")
        source = fields[1]
        if source.startswith("demos/") and source.endswith(".c"):
            sources.add(source)
    return sources


def require_entry(entry: dict[str, object], source_kind: str) -> None:
    source = entry.get("source")
    require(isinstance(source, str) and source, f"{source_kind} entry has no source")
    require((ROOT / source).exists(), f"{source_kind} source is missing: {source}")
    features = entry.get("required_features")
    require(isinstance(features, list) and features, f"{source} must list required_features")
    require(all(isinstance(feature, str) and feature for feature in features), f"{source} has invalid required_features")
    status = entry.get("status")
    require(status in {"blocked_by_features", "active", "replaced_by_llvm"}, f"{source} has invalid status {status}")
    if status in {"blocked_by_features", "replaced_by_llvm"}:
        require(features, f"{source} is non-active but has no required_features")
        require("rtl_gate" not in entry, f"{source} should not name an RTL gate unless it is active")
    else:
        rtl_gate = entry.get("rtl_gate")
        require(isinstance(rtl_gate, str) and rtl_gate, f"{source} active entry must name rtl_gate")
        require((ROOT / rtl_gate).exists(), f"{source} rtl_gate is missing: {rtl_gate}")
        comparison = entry.get("comparison")
        require(
            comparison == "retire_trace_and_final_state",
            f"{source} active entry must compare retire_trace_and_final_state",
        )


def require_typed_retire_gate(gate_text: str, source: str) -> None:
    require("retire_required_fields" in gate_text, f"{source} gate must validate typed retire fields")
    for field in ("tile_id", "pid", "tid", "domain_id", "domain_gen", "operand_rd", "operand_rs1", "operand_rs2", "operand_rs3", "operand_imm", "result_valid", "result_reg", "result_value", "errno", "status", "event_id", "fault_id"):
        require(f'"{field}"' in gate_text, f"{source} gate must require typed retire field {field}")


def effective_gate_text(gate_path: str) -> str:
    gate_text = text(ROOT / gate_path)
    if gate_path != "scripts/run_rtl_top_program_smoke.sh" and "scripts/run_rtl_top_program_smoke.sh" in gate_text:
        gate_text += "\n" + text(ROOT / "scripts/run_rtl_top_program_smoke.sh")
    return gate_text


def main() -> None:
    manifest = json.loads(text(MANIFEST))
    manifest_runner = text(ROOT / "scripts/run_rtl_top_program_manifest.sh")
    require("entry['rtl_gate']" in manifest_runner, "manifest runner must dispatch active entries through their rtl_gate")
    require("llvm_mc_programs" in manifest_runner, "manifest runner must include LLVM MC program entries")
    require("llvm_clang_programs" in manifest_runner, "manifest runner must include LLVM clang program entries")
    require("llvm_linked_programs" in manifest_runner, "manifest runner must include LLVM linked program entries")
    smoke_gate_text = text(ROOT / "scripts/run_rtl_top_program_smoke.sh")
    require("RTL_M1_TOP_PRE_STATE" in smoke_gate_text, "shared top-level comparator must consume M1 pre-state projections")
    require("RTL_M1_TOP_STATE" in smoke_gate_text, "shared top-level comparator must consume M1 post-state projections")
    require("check_top_m1_projection_matches_commit" in smoke_gate_text, "shared top-level comparator must check M1 commit/projection alignment")
    require('f"{prefix}_domain_id"' in smoke_gate_text, "shared top-level comparator must check M1 projection domain fields")
    require("CapRevoke" in smoke_gate_text, "shared top-level comparator must check CAP_REVOKE M1 behavior")
    require("accepted without REVOKE right" in smoke_gate_text, "shared top-level comparator must reject CAP_REVOKE without authority")
    require("did not publish revoked generation" in smoke_gate_text, "shared top-level comparator must require CAP_REVOKE revoked-generation evidence")
    require("left root authority live" in smoke_gate_text, "shared top-level comparator must require CAP_REVOKE authority removal")
    require("RTL_EVENT" in smoke_gate_text, "shared top-level comparator must consume top-level event records")
    require("cross_tile_wake" in smoke_gate_text, "shared top-level comparator must check cross-tile wake events")
    require("lnp64_m1_state_projection_t" in text(ROOT / "rtl/sim/lnp64_top_program_tb.sv"), "top-program testbench must emit schema-owned M1 state projections")
    require(manifest.get("schema") == "lnp64_top_level_program_tests_v1", "unexpected manifest schema")
    require(manifest.get("stage") == "feature_gated_plan", "manifest must be a feature-gated plan")
    require(manifest.get("top") == "rtl/top/lnp64_top.sv", "manifest must target lnp64_top")
    require((ROOT / "rtl/top/lnp64_top.sv").exists(), "lnp64_top is missing")

    flat_hex_entries = manifest.get("flat_hex_programs")
    llvm_mc_entries = manifest.get("llvm_mc_programs")
    llvm_clang_entries = manifest.get("llvm_clang_programs")
    llvm_linked_entries = manifest.get("llvm_linked_programs")
    compiler_flat_entries = manifest.get("compiler_flat_programs")
    assembly_entries = manifest.get("assembly_programs")
    compiler_entries = manifest.get("compiler_generated_programs")
    require(isinstance(flat_hex_entries, list) and flat_hex_entries, "missing flat_hex_programs")
    require(isinstance(llvm_mc_entries, list) and llvm_mc_entries, "missing llvm_mc_programs")
    require(isinstance(llvm_clang_entries, list) and llvm_clang_entries, "missing llvm_clang_programs")
    require(isinstance(llvm_linked_entries, list) and llvm_linked_entries, "missing llvm_linked_programs")
    require(isinstance(compiler_flat_entries, list) and compiler_flat_entries, "missing compiler_flat_programs")
    require(isinstance(assembly_entries, list) and assembly_entries, "missing assembly_programs")
    require(isinstance(compiler_entries, list) and compiler_entries, "missing compiler_generated_programs")
    active_flat_hex = 0
    for entry in flat_hex_entries:
        require(isinstance(entry, dict), "flat hex entry must be an object")
        require_entry(entry, "flat hex")
        if entry.get("status") == "active":
            active_flat_hex += 1
            gate_text = effective_gate_text(str(entry["rtl_gate"]))
            require("program_input=" in gate_text, f"{entry['source']} active gate must accept a program input")
            generated_flat_hex = entry.get("generated_flat_hex")
            require(
                isinstance(generated_flat_hex, str) and generated_flat_hex.endswith(".hex"),
                f"{entry['source']} active entry must name generated_flat_hex",
            )
            require((ROOT / generated_flat_hex).exists(), f"{entry['source']} generated_flat_hex is missing")
            if str(entry["source"]).endswith(".hex"):
                require(
                    generated_flat_hex == entry["source"],
                    f"{entry['source']} raw hex entry must use itself as generated_flat_hex",
                )
            else:
                require("asm-flat-exec" in gate_text, f"{entry['source']} active gate must assemble source to flat hex")
            require("RTL_RETIRE" in gate_text and "EMULATOR_RETIRE" in gate_text, f"{entry['source']} gate must compare retire traces")
            require_typed_retire_gate(gate_text, str(entry["source"]))
            require("RTL_FINAL" in gate_text and "EMULATOR_FINAL" in gate_text, f"{entry['source']} gate must compare final state")
            require('"regs"' in gate_text, f"{entry['source']} gate must compare final register file")
            require('"errno"' in gate_text, f"{entry['source']} gate must compare final errno")
            require('"mem_checksum"' in gate_text, f"{entry['source']} gate must compare final memory checksum")
    require(active_flat_hex >= 1, "manifest must keep at least one active top-level flat hex program")
    active_llvm_mc = 0
    for entry in llvm_mc_entries:
        require(isinstance(entry, dict), "LLVM MC entry must be an object")
        require_entry(entry, "LLVM MC")
        if entry.get("status") == "active":
            active_llvm_mc += 1
            gate_text = text(ROOT / str(entry["rtl_gate"]))
            require("llvm-mc" in gate_text, f"{entry['source']} active gate must assemble with llvm-mc")
            require("llvm-objdump" in gate_text, f"{entry['source']} active gate must decode object bytes with llvm-objdump")
            require(".dump" in gate_text, f"{entry['source']} active gate must create an objdump dump")
            require(
                "scripts/run_rtl_top_program_smoke.sh" in gate_text,
                f"{entry['source']} active gate must feed bytes to the shared top-level comparator",
            )
            smoke_text = text(ROOT / "scripts/run_rtl_top_program_smoke.sh")
            require(
                "scripts/llvm_objdump_to_flat_hex.py" in smoke_text,
                f"{entry['source']} shared comparator must convert LLVM objdump bytes to flat hex",
            )
            require("RTL_RETIRE" in smoke_text and "EMULATOR_RETIRE" in smoke_text, f"{entry['source']} gate must compare retire traces")
            require_typed_retire_gate(smoke_text, str(entry["source"]))
            require("RTL_FINAL" in smoke_text and "EMULATOR_FINAL" in smoke_text, f"{entry['source']} gate must compare final state")
            require('"regs"' in smoke_text, f"{entry['source']} gate must compare final register file")
            require('"errno"' in smoke_text, f"{entry['source']} gate must compare final errno")
            require('"mem_checksum"' in smoke_text, f"{entry['source']} gate must compare final memory checksum")
    require(active_llvm_mc >= 1, "manifest must keep at least one active LLVM MC object-byte top-level program")
    active_llvm_clang = 0
    for entry in llvm_clang_entries:
        require(isinstance(entry, dict), "LLVM clang entry must be an object")
        require_entry(entry, "LLVM clang")
        if entry.get("status") == "active":
            active_llvm_clang += 1
            gate_text = text(ROOT / str(entry["rtl_gate"]))
            require("--target=lnp64-unknown-none" in gate_text, f"{entry['source']} active gate must compile for LNP64 clang")
            require("llvm-objdump" in gate_text, f"{entry['source']} active gate must decode object bytes with llvm-objdump")
            require("--wrap-call-exit-r1" in gate_text, f"{entry['source']} active gate must use an explicit top-level exit harness")
            require(
                "scripts/run_rtl_top_program_smoke.sh" in gate_text,
                f"{entry['source']} active gate must feed bytes to the shared top-level comparator",
            )
            converter_text = text(ROOT / "scripts/llvm_objdump_to_flat_hex.py")
            require("wrap-call-exit-r1" in converter_text, f"{entry['source']} converter must support explicit call/exit harnessing")
            smoke_text = text(ROOT / "scripts/run_rtl_top_program_smoke.sh")
            require("RTL_RETIRE" in smoke_text and "EMULATOR_RETIRE" in smoke_text, f"{entry['source']} gate must compare retire traces")
            require_typed_retire_gate(smoke_text, str(entry["source"]))
            require("RTL_FINAL" in smoke_text and "EMULATOR_FINAL" in smoke_text, f"{entry['source']} gate must compare final state")
            require('"regs"' in smoke_text, f"{entry['source']} gate must compare final register file")
            require('"errno"' in smoke_text, f"{entry['source']} gate must compare final errno")
            require('"mem_checksum"' in smoke_text, f"{entry['source']} gate must compare final memory checksum")
    require(active_llvm_clang >= 1, "manifest must keep at least one active LLVM clang object-byte top-level program")
    active_llvm_linked = 0
    for entry in llvm_linked_entries:
        require(isinstance(entry, dict), "LLVM linked entry must be an object")
        require_entry(entry, "LLVM linked")
        if entry.get("status") == "active":
            active_llvm_linked += 1
            gate_text = text(ROOT / str(entry["rtl_gate"]))
            require("--target=lnp64-unknown-none" in gate_text, f"{entry['source']} active gate must compile for LNP64 clang")
            require("llvm-mc" in gate_text, f"{entry['source']} active gate must be able to assemble the flat startup object")
            require("-flavor gnu" in gate_text and "-m elf64lnp64" in gate_text, f"{entry['source']} active gate must link with LNP64 lld")
            require("elf-plan" in gate_text, f"{entry['source']} active gate must validate the linked ELF loader plan")
            require("elf-flat-exec" in gate_text, f"{entry['source']} active gate must export the linked ELF to a top-level flat image")
            require(
                "scripts/run_rtl_top_program_smoke.sh" in gate_text,
                f"{entry['source']} active gate must feed the linked image to the shared top-level comparator",
            )
            smoke_text = text(ROOT / "scripts/run_rtl_top_program_smoke.sh")
            require("RTL_RETIRE" in smoke_text and "EMULATOR_RETIRE" in smoke_text, f"{entry['source']} gate must compare retire traces")
            require_typed_retire_gate(smoke_text, str(entry["source"]))
            require("RTL_FINAL" in smoke_text and "EMULATOR_FINAL" in smoke_text, f"{entry['source']} gate must compare final state")
            require('"regs"' in smoke_text, f"{entry['source']} gate must compare final register file")
            require('"errno"' in smoke_text, f"{entry['source']} gate must compare final errno")
            require('"mem_checksum"' in smoke_text, f"{entry['source']} gate must compare final memory checksum")
    require(active_llvm_linked >= 1, "manifest must keep at least one active LLVM linked top-level program")
    active_compiler_flat = 0
    for entry in compiler_flat_entries:
        require(isinstance(entry, dict), "compiler flat entry must be an object")
        require_entry(entry, "compiler flat")
        if entry.get("status") == "active":
            active_compiler_flat += 1
    require(active_compiler_flat == 0, "compiler-flat toy C entries should stay retired once linked LLVM replacements exist")
    for entry in assembly_entries:
        require(isinstance(entry, dict), "assembly entry must be an object")
        require_entry(entry, "assembly")
        if entry.get("status") == "active":
            gate_text = text(ROOT / str(entry["rtl_gate"]))
            require("program_input=" in gate_text, f"{entry['source']} active gate must accept a program input")
            require("asm-flat-exec" in gate_text, f"{entry['source']} active gate must assemble source to flat hex")
            require("RTL_RETIRE" in gate_text and "EMULATOR_RETIRE" in gate_text, f"{entry['source']} gate must compare retire traces")
            require_typed_retire_gate(gate_text, str(entry["source"]))
            require("RTL_FINAL" in gate_text and "EMULATOR_FINAL" in gate_text, f"{entry['source']} gate must compare final state")
            require('"regs"' in gate_text, f"{entry['source']} gate must compare final register file")
            require('"errno"' in gate_text, f"{entry['source']} gate must compare final errno")
            require('"mem_checksum"' in gate_text, f"{entry['source']} gate must compare final memory checksum")
    for entry in compiler_entries:
        require(isinstance(entry, dict), "compiler entry must be an object")
        require_entry(entry, "compiler")
        generated = entry.get("generated_assembly")
        require(isinstance(generated, str) and generated.endswith(".s"), f"{entry['source']} must name generated assembly")
        if entry.get("status") == "active":
            pass

    manifest_asm = {entry["source"] for entry in assembly_entries}
    actual_asm = {str(path.relative_to(ROOT)) for path in sorted((ROOT / "demos").glob("*.s"))}
    require(manifest_asm == actual_asm, f"demos/*.s coverage drifted: actual={sorted(actual_asm)} manifest={sorted(manifest_asm)}")

    manifest_c = {entry["source"] for entry in compiler_entries}
    expected_c = llvm_bootstrap_demo_c_sources(text(LLVM_BOOTSTRAP))
    require(
        manifest_c == expected_c,
        f"compiler demo coverage drifted: expected={sorted(expected_c)} manifest={sorted(manifest_c)}",
    )

    linked_sources = {
        str(entry["source"]): entry
        for entry in llvm_linked_entries
        if entry.get("status") == "active"
    }
    active_toy_sources = {
        str(entry["source"])
        for entry in compiler_flat_entries + compiler_entries
        if entry.get("status") == "active"
    }
    retired_or_active_toy_sources = {
        str(entry["source"])
        for entry in compiler_flat_entries + compiler_entries
        if entry.get("status") in {"active", "replaced_by_llvm"}
    }
    require(not active_toy_sources, f"toy C sources remain active after linked LLVM replacement: {sorted(active_toy_sources)}")
    missing_replacements = sorted(retired_or_active_toy_sources - set(LINKED_REPLACEMENTS))
    require(
        not missing_replacements,
        f"retired toy C sources lack linked LLVM replacements: {missing_replacements}",
    )
    for toy_source in sorted(retired_or_active_toy_sources):
        linked_source, required_feature = LINKED_REPLACEMENTS[toy_source]
        linked_entry = linked_sources.get(linked_source)
        require(
            linked_entry is not None,
            f"{toy_source} replacement {linked_source} must be an active LLVM linked entry",
        )
        linked_features = linked_entry.get("required_features")
        require(
            isinstance(linked_features, list) and required_feature in linked_features,
            f"{toy_source} replacement {linked_source} must advertise {required_feature}",
        )
        require(
            linked_entry.get("rtl_gate") == "scripts/run_rtl_top_linked_llvm_smoke.sh",
            f"{toy_source} replacement {linked_source} must use the linked LLVM smoke gate",
        )

    requirements = manifest.get("recurring_gate_requirements", [])
    require(isinstance(requirements, list) and len(requirements) >= 3, "missing recurring gate requirements")
    print("rtl top-level program manifest ok")


if __name__ == "__main__":
    main()

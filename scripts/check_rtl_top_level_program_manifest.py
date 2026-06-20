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
REQUIRED_LINKED_COVERAGE = {
    "tests/rtl/programs/top_linked_main.c": {"startup_call_main"},
    "tests/rtl/programs/top_linked_loop_branch.c": {"branch", "call_return"},
    "tests/rtl/programs/top_linked_factorial_mul.c": {"mul"},
    "tests/rtl/programs/top_linked_bitwise_shift.c": {"bitwise_alu", "shift_alu"},
    "tests/rtl/programs/top_linked_divrem.c": {"unsigned_division", "signed_division"},
    "tests/rtl/programs/top_linked_byte_array.c": {"byte_load_store"},
    "tests/rtl/programs/top_linked_heap_byte_lanes.c": {"heap"},
    "tests/rtl/programs/top_linked_allocator_native.c": {"heap", "free"},
    "tests/rtl/programs/top_linked_factorial_native.c": {"push_pull"},
    "tests/rtl/programs/top_linked_fibonacci_native.c": {"call_return"},
    "tests/rtl/programs/top_linked_hello_native.c": {"push_pull"},
    "tests/rtl/programs/top_linked_json_parser_native.c": {"heap", "free"},
    "tests/rtl/programs/top_linked_clone_join.c": {"thread_join"},
    "tests/rtl/programs/top_linked_rot13_native.c": {"push_pull", "free"},
}
M1_TOP_LEVEL_COVERED_KEYS = {
    "cap_dup": ("LNP64_OP_CAP_DUP", "LNP64_M1_COMMIT_CAP_DUP"),
    "cap_send": ("LNP64_OP_CAP_SEND", "LNP64_M1_COMMIT_CAP_SEND"),
    "cap_recv": ("LNP64_OP_CAP_RECV", "LNP64_M1_COMMIT_CAP_RECV"),
    "cap_revoke": ("LNP64_OP_CAP_REVOKE", "LNP64_M1_COMMIT_CAP_REVOKE"),
    "reject_stale": ("LNP64_OP_PULL", "LNP64_M1_COMMIT_REJECT_STALE"),
    "push": ("LNP64_OP_PUSH", "LNP64_M1_COMMIT_PUSH"),
    "pull": ("LNP64_OP_PULL", "LNP64_M1_COMMIT_PULL"),
    "reject_full": ("LNP64_OP_PUSH", "LNP64_M1_COMMIT_REJECT_FULL"),
    "object_create": ("LNP64_OP_OBJECT_CTL", "LNP64_M1_COMMIT_OBJECT_CREATE"),
    "cap_dup_denied": ("LNP64_OP_CAP_DUP", "LNP64_M1_COMMIT_CAP_DUP_DENIED"),
}
M1_STANDALONE_UNTIL_S1_KEYS = {}


def fail(message: str) -> None:
    raise SystemExit(f"rtl top-level program manifest check failed: {message}")


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def text(path: Path) -> str:
    require(path.exists(), f"missing file {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def require_entry(entry: dict[str, object], source_kind: str) -> None:
    source = entry.get("source")
    require(isinstance(source, str) and source, f"{source_kind} entry has no source")
    require((ROOT / source).exists(), f"{source_kind} source is missing: {source}")
    features = entry.get("required_features")
    require(isinstance(features, list) and features, f"{source} must list required_features")
    require(all(isinstance(feature, str) and feature for feature in features), f"{source} has invalid required_features")
    status = entry.get("status")
    require(status in {"blocked_by_features", "active"}, f"{source} has invalid status {status}")
    if status == "blocked_by_features":
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


def require_m1_top_level_refinement_contract(manifest: dict[str, object]) -> None:
    contract = manifest.get("m1_top_level_refinement")
    require(isinstance(contract, dict), "manifest must document m1_top_level_refinement")
    require(
        contract.get("stage") == "top_level_cap_and_queue_ops_retired_instruction_coupled",
        "M1 top-level refinement stage must state cap-and-queue coupling",
    )
    claim = contract.get("claim")
    require(isinstance(claim, str) and "not T4" in claim, "M1 top-level claim must keep the non-T4 gap explicit")
    require(contract.get("gate") == "scripts/run_rtl_m1_refinement_docker.sh", "M1 top-level contract must name the Docker refinement gate")
    require(contract.get("checker") == "scripts/check_rtl_top_level_program_manifest.py", "M1 top-level contract must name this checker")
    witness_artifact = contract.get("generated_witness_artifact")
    require(isinstance(witness_artifact, dict), "M1 top-level contract must document the generated witness artifact")
    require(
        witness_artifact.get("env") == "LNP64_RTL_TOP_M1_WITNESS_OUT",
        "M1 generated witness artifact env drifted",
    )
    require(
        witness_artifact.get("schema") == "lnp64_top_m1_refinement_witness_v1",
        "M1 generated witness artifact schema drifted",
    )
    require(
        witness_artifact.get("producer") == "scripts/run_rtl_top_program_smoke.sh",
        "M1 generated witness artifact producer drifted",
    )
    require(
        witness_artifact.get("shared_mirror") == "formal/m1_top_refinement.py",
        "M1 generated witness artifact must name the shared executable refinement mirror",
    )
    require(
        witness_artifact.get("consumer") == "scripts/check_rtl_top_m1_witness.py",
        "M1 generated witness artifact must name the offline witness consumer",
    )
    require(
        witness_artifact.get("consumer_self_test") == "scripts/test_rtl_top_m1_witness_checker.py",
        "M1 generated witness artifact must name the offline witness consumer self-test",
    )
    require(
        witness_artifact.get("gate") == "scripts/run_rtl_top_m1_witness_gate.sh",
        "M1 generated witness artifact must name the produce-and-check gate",
    )
    lean_df = witness_artifact.get("lean_decode_faithfulness")
    require(isinstance(lean_df, dict), "M1 witness artifact must document the Lean decode-faithfulness artifact")
    require(
        lean_df.get("generator") == "scripts/gen_m1_witness_lean.py",
        "M1 Lean decode-faithfulness must name the generator",
    )
    require(
        lean_df.get("gate") == "scripts/run_rtl_m1_lean_witness_gate.sh",
        "M1 Lean decode-faithfulness must name the gate",
    )
    require(
        lean_df.get("model") == "formal/M1TransitionInvariantModel.lean",
        "M1 Lean decode-faithfulness must name the model",
    )
    require(
        lean_df.get("tactic") == "decide",
        "M1 Lean decode-faithfulness must use the kernel decide tactic",
    )
    for artifact_file in (
        "formal/m1_top_refinement.py",
        "scripts/check_rtl_top_m1_witness.py",
        "scripts/test_rtl_top_m1_witness_checker.py",
        "scripts/run_rtl_top_m1_witness_gate.sh",
        "scripts/gen_m1_witness_lean.py",
        "scripts/run_rtl_m1_lean_witness_gate.sh",
    ):
        require((ROOT / artifact_file).exists(), f"M1 witness artifact file is missing: {artifact_file}")
    lean_gate_text = text(ROOT / "scripts/run_rtl_m1_lean_witness_gate.sh")
    require(
        "scripts/gen_m1_witness_lean.py" in lean_gate_text,
        "M1 Lean witness gate must run the generator",
    )
    require(
        "native_decide" in lean_gate_text and "axiom|sorry|admit" in lean_gate_text,
        "M1 Lean witness gate must reject native_decide/axiom/sorry/admit in generated proofs",
    )
    witness_gate_text = text(ROOT / "scripts/run_rtl_top_m1_witness_gate.sh")
    require(
        "scripts/check_rtl_top_m1_witness.py" in witness_gate_text,
        "M1 witness gate must run the offline witness consumer",
    )
    require(
        "scripts/test_rtl_top_m1_witness_checker.py" in witness_gate_text,
        "M1 witness gate must run the offline witness consumer self-test",
    )
    m1_docker_gate_witness_text = text(ROOT / "scripts/run_rtl_m1_refinement_docker.sh")
    require(
        "scripts/run_rtl_top_m1_witness_gate.sh" in m1_docker_gate_witness_text,
        "M1 Docker refinement gate must run the witness produce-and-check gate",
    )
    require(
        "scripts/run_rtl_m1_lean_witness_gate.sh" in m1_docker_gate_witness_text,
        "M1 Docker refinement gate must run the Lean witness decode-faithfulness gate",
    )
    remaining_gap = contract.get("remaining_t4_gap")
    require(
        isinstance(remaining_gap, str) and "formal RTL-to-Lean bit-refinement proof artifact" in remaining_gap,
        "M1 top-level contract must state the remaining RTL-to-Lean bit-refinement gap",
    )

    schema = json.loads(text(ROOT / "rtl/schema/lnp64_shared_schema.json"))
    schema_ops = {
        entry["key"]: entry
        for entry in schema["m1_typed_commit_contract"]["op_mappings"]
    }
    expected_keys = set(M1_TOP_LEVEL_COVERED_KEYS) | set(M1_STANDALONE_UNTIL_S1_KEYS)
    require(
        set(schema_ops) == expected_keys,
        f"M1 top-level contract partition drifted from schema op mappings: schema={sorted(schema_ops)} expected={sorted(expected_keys)}",
    )

    covered = contract.get("covered_real_instruction_ops")
    standalone = contract.get("standalone_until_s1_hooks")
    require(isinstance(covered, list) and covered, "M1 contract must list covered top-level ops")
    require(isinstance(standalone, list), "M1 contract must list standalone-until-S1 ops")

    def require_ops(entries: list[object], expected: dict[str, tuple[str, str]], label: str) -> None:
        by_key = {}
        for raw_entry in entries:
            require(isinstance(raw_entry, dict), f"M1 {label} entry must be an object")
            key = raw_entry.get("key")
            require(isinstance(key, str), f"M1 {label} entry has invalid key")
            require(key not in by_key, f"M1 {label} duplicates key {key}")
            by_key[key] = raw_entry
        require(set(by_key) == set(expected), f"M1 {label} keys drifted: saw={sorted(by_key)} expected={sorted(expected)}")
        for key, (arch_opcode, commit_op) in expected.items():
            entry = by_key[key]
            require(entry.get("arch_opcode") == arch_opcode, f"M1 {label} {key} arch opcode drifted")
            require(entry.get("commit_op") == commit_op, f"M1 {label} {key} commit op drifted")
            require(
                entry.get("lean_step") == schema_ops[key]["lean_transition"],
                f"M1 {label} {key} Lean step drifted from shared schema",
            )
            if label == "standalone-until-S1":
                missing_hook = entry.get("missing_hook")
                missing_hook_is_explicit = (
                    isinstance(missing_hook, str)
                    and "does not yet emit schema-owned M1" in missing_hook
                ) or (
                    key == "cap_dup_denied"
                    and isinstance(missing_hook, str)
                    and "not yet emitted" in missing_hook
                )
                require(
                    missing_hook_is_explicit,
                    f"M1 standalone op {key} must explicitly state the missing top-level hook",
                )

    require_ops(covered, M1_TOP_LEVEL_COVERED_KEYS, "covered")
    require_ops(standalone, M1_STANDALONE_UNTIL_S1_KEYS, "standalone-until-S1")

    top_text = text(ROOT / "rtl/top/lnp64_top.sv")
    smoke_text = text(ROOT / "scripts/run_rtl_top_program_smoke.sh")
    m1_docker_gate_text = text(ROOT / "scripts/run_rtl_m1_refinement_docker.sh")
    require("LNP64_RTL_TOP_M1_WITNESS_OUT" in smoke_text, "top-level smoke must expose generated M1 witness artifact output")
    require("lnp64_top_m1_refinement_witness_v1" in smoke_text, "top-level smoke must label generated M1 witness artifacts")
    require("records_sha256" in smoke_text, "top-level smoke must hash generated M1 witness records")
    require("commit_bits" in smoke_text, "top-level smoke must include packed commit bits in generated M1 witnesses")
    require("pre_state_bits" in smoke_text, "top-level smoke must include packed pre-state bits in generated M1 witnesses")
    require("post_state_bits" in smoke_text, "top-level smoke must include packed post-state bits in generated M1 witnesses")
    require("top_pipe_push_pull.s" in m1_docker_gate_text, "M1 Docker gate must include dynamic pipe PUSH/PULL top-level coverage")
    require("top_pipe_static_push_pull.s" in m1_docker_gate_text, "M1 Docker gate must include static pipe PUSH/PULL top-level coverage")
    for arch_opcode, commit_op in M1_TOP_LEVEL_COVERED_KEYS.values():
        require(arch_opcode in top_text, f"lnp64_top must map covered M1 opcode {arch_opcode}")
        require(commit_op in top_text, f"lnp64_top must emit covered M1 commit op {commit_op}")
        require(arch_opcode in smoke_text, f"top-level comparator must map covered M1 opcode {arch_opcode}")
        require(commit_op in smoke_text, f"top-level comparator must decode covered M1 commit op {commit_op}")
    for key, (arch_opcode, commit_op) in M1_STANDALONE_UNTIL_S1_KEYS.items():
        require(arch_opcode in smoke_text, f"shared comparator must know standalone-related arch opcode {arch_opcode} for {key}")
        require(schema_ops[key]["sv"] == commit_op, f"schema must retain standalone M1 commit op {commit_op}")
        if key not in {"cap_dup_denied"}:
            require(
                commit_op not in top_text,
                f"standalone M1 op {key} is now present in lnp64_top; move it to covered_real_instruction_ops",
            )


def main() -> None:
    manifest = json.loads(text(MANIFEST))
    manifest_runner = text(ROOT / "scripts/run_rtl_top_program_manifest.sh")
    require("entry['rtl_gate']" in manifest_runner, "manifest runner must dispatch active entries through their rtl_gate")
    require("llvm_mc_programs" in manifest_runner, "manifest runner must include LLVM MC program entries")
    require("llvm_clang_programs" in manifest_runner, "manifest runner must include LLVM clang program entries")
    require("llvm_linked_programs" in manifest_runner, "manifest runner must include LLVM linked program entries")
    smoke_gate_text = text(ROOT / "scripts/run_rtl_top_program_smoke.sh")
    require("RTL_FABRIC_CMD" in smoke_gate_text, "shared top-level comparator must consume fabric command provenance records")
    require("check_fabric_cmd_records" in smoke_gate_text, "shared top-level comparator must validate fabric command provenance records")
    require("RTL_M1_TOP_PRE_STATE" in smoke_gate_text, "shared top-level comparator must consume M1 pre-state projections")
    require("RTL_M1_TOP_STATE" in smoke_gate_text, "shared top-level comparator must consume M1 post-state projections")
    # The M1 refinement relation now lives in the shared executable mirror so the
    # producer smoke and the offline witness consumer cannot drift.
    require(
        "from m1_top_refinement import" in smoke_gate_text,
        "shared top-level comparator must import the M1 refinement mirror",
    )
    require(
        "check_top_m1_refinement_step" in smoke_gate_text,
        "shared top-level comparator must run the shared M1 refinement step",
    )
    mirror_text = text(ROOT / "formal/m1_top_refinement.py")
    require("check_top_m1_projection_matches_commit" in mirror_text, "shared M1 refinement mirror must check commit/projection alignment")
    require('f"{prefix}_domain_id"' in mirror_text, "shared M1 refinement mirror must check projection domain fields")
    require("CapRevoke" in mirror_text, "shared M1 refinement mirror must check CAP_REVOKE M1 behavior")
    require("accepted without REVOKE right" in mirror_text, "shared M1 refinement mirror must reject CAP_REVOKE without authority")
    require("did not publish revoked generation" in mirror_text, "shared M1 refinement mirror must require CAP_REVOKE revoked-generation evidence")
    require("left root authority live" in mirror_text, "shared M1 refinement mirror must require CAP_REVOKE authority removal")
    require("RTL_EVENT" in smoke_gate_text, "shared top-level comparator must consume top-level event records")
    require("cross_tile_wake" in smoke_gate_text, "shared top-level comparator must check cross-tile wake events")
    require("scheduler_wake_issue" in smoke_gate_text, "shared top-level comparator must tie cross-tile events to scheduler wake issue")
    top_program_tb_text = text(ROOT / "rtl/sim/lnp64_top_program_tb.sv")
    require("scheduler_wake_issue" in top_program_tb_text, "top-program testbench must emit scheduler wake issue in event records")
    require("dut.sched_wake_issue_valid" in top_program_tb_text, "top-program event trace must use the scheduler wake signal")
    require("lnp64_m1_state_projection_t" in top_program_tb_text, "top-program testbench must emit schema-owned M1 state projections")
    require(manifest.get("schema") == "lnp64_top_level_program_tests_v1", "unexpected manifest schema")
    require(manifest.get("stage") == "feature_gated_plan", "manifest must be a feature-gated plan")
    require(manifest.get("top") == "rtl/top/lnp64_top.sv", "manifest must target lnp64_top")
    require((ROOT / "rtl/top/lnp64_top.sv").exists(), "lnp64_top is missing")
    require_m1_top_level_refinement_contract(manifest)

    flat_hex_entries = manifest.get("flat_hex_programs")
    llvm_mc_entries = manifest.get("llvm_mc_programs")
    llvm_clang_entries = manifest.get("llvm_clang_programs")
    llvm_linked_entries = manifest.get("llvm_linked_programs")
    assembly_entries = manifest.get("assembly_programs")
    require(isinstance(flat_hex_entries, list) and flat_hex_entries, "missing flat_hex_programs")
    require(isinstance(llvm_mc_entries, list) and llvm_mc_entries, "missing llvm_mc_programs")
    require(isinstance(llvm_clang_entries, list) and llvm_clang_entries, "missing llvm_clang_programs")
    require(isinstance(llvm_linked_entries, list) and llvm_linked_entries, "missing llvm_linked_programs")
    require(isinstance(assembly_entries, list) and assembly_entries, "missing assembly_programs")
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
    linked_sources = {
        str(entry["source"]): entry
        for entry in llvm_linked_entries
        if entry.get("status") == "active"
    }
    for linked_source, required_features in REQUIRED_LINKED_COVERAGE.items():
        linked_entry = linked_sources.get(linked_source)
        require(linked_entry is not None, f"{linked_source} must be active LLVM linked coverage")
        linked_features = linked_entry.get("required_features")
        require(isinstance(linked_features, list), f"{linked_source} must list required_features")
        for required_feature in required_features:
            require(
                required_feature in linked_features,
                f"{linked_source} must advertise required feature {required_feature}",
            )
        require(
            linked_entry.get("rtl_gate") == "scripts/run_rtl_top_linked_llvm_smoke.sh",
            f"{linked_source} must use the linked LLVM smoke gate",
        )
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

    manifest_asm = {entry["source"] for entry in assembly_entries}
    actual_asm = {str(path.relative_to(ROOT)) for path in sorted((ROOT / "demos").glob("*.s"))}
    require(manifest_asm == actual_asm, f"demos/*.s coverage drifted: actual={sorted(actual_asm)} manifest={sorted(manifest_asm)}")

    bootstrap_text = text(LLVM_BOOTSTRAP)
    for demo_case in ("hello", "arithmetic", "memory", "calls", "json_parser", "rot13", "ping_pong"):
        require(demo_case in bootstrap_text, f"LLVM bootstrap manifest must retain {demo_case} demo coverage")

    requirements = manifest.get("recurring_gate_requirements", [])
    require(isinstance(requirements, list) and len(requirements) >= 3, "missing recurring gate requirements")
    print("rtl top-level program manifest ok")


if __name__ == "__main__":
    main()

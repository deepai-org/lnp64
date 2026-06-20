#!/usr/bin/env python3
"""Validate human-auditable theorem-to-RTL coupling evidence."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path(
    os.environ.get(
        "LNP64_THEOREM_RTL_COUPLING_MANIFEST",
        str(ROOT / "formal/theorem_rtl_coupling_manifest.json"),
    )
)
INDEX = Path(
    os.environ.get(
        "LNP64_THEOREM_RTL_COUPLING_INDEX",
        str(ROOT / "formal/theorem_rtl_coupling_index.md"),
    )
)
TOP_LEVEL_PROGRAM_MANIFEST = ROOT / "tests/rtl/top_level_program_manifest.json"
RTL_PROOF_GATES = ROOT / "scripts/run_rtl_proof_gates.sh"
ALLOWED_TRUST_LEVELS = {"T0", "T1", "T2", "T3", "T4", "T5"}
ALLOWED_ARTIFACT_LEVELS = {
    "coverage",
    "bounded_witness",
    "transition_invariant",
    "refinement",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def root_path(name: str) -> Path:
    return ROOT / name


def theorem_exists(path: Path, theorem: str) -> bool:
    return re.search(rf"(?m)^theorem\s+{re.escape(theorem)}\b", read_text(path)) is not None


def module_exists(path: Path, module: str) -> bool:
    return re.search(rf"(?m)^\s*module\s+{re.escape(module)}\b", read_text(path)) is not None


def check_m1_top_level_contract(claim: dict) -> None:
    claim_id = claim.get("id")
    if claim_id != "no_forged_authority":
        return
    manifest = json.loads(check_file(TOP_LEVEL_PROGRAM_MANIFEST, "M1 top-level program manifest"))
    contract = manifest.get("m1_top_level_refinement")
    require(isinstance(contract, dict), "no_forged_authority: missing M1 top-level refinement contract")
    covered = contract.get("covered_real_instruction_ops")
    standalone = contract.get("standalone_until_s1_hooks")
    require(isinstance(covered, list), "no_forged_authority: M1 top-level covered ops must be a list")
    require(standalone == [], "no_forged_authority: M1 top-level standalone hooks must stay empty")
    covered_keys = {entry.get("key") for entry in covered if isinstance(entry, dict)}
    required = {
        "cap_dup",
        "cap_send",
        "cap_recv",
        "cap_revoke",
        "reject_stale",
        "push",
        "pull",
        "reject_full",
        "object_create",
        "cap_dup_denied",
    }
    missing = sorted(required - covered_keys)
    require(not missing, f"no_forged_authority: M1 top-level contract missing covered op(s): {missing}")
    remaining_gap = contract.get("remaining_t4_gap")
    require(
        isinstance(remaining_gap, str) and "RTL-to-Lean bit-refinement" in remaining_gap,
        "no_forged_authority: M1 top-level contract must keep the T4 bit-refinement gap explicit",
    )

    witness = contract.get("generated_witness_artifact")
    require(isinstance(witness, dict), "no_forged_authority: missing generated witness artifact contract")
    require(
        witness.get("consumer") == "scripts/check_rtl_top_m1_witness.py",
        "no_forged_authority: witness artifact must name the offline consumer",
    )
    require(
        witness.get("shared_mirror") == "formal/m1_top_refinement.py",
        "no_forged_authority: witness artifact must name the shared refinement mirror",
    )
    lean_df = witness.get("lean_decode_faithfulness")
    require(isinstance(lean_df, dict), "no_forged_authority: witness artifact must document Lean decode-faithfulness")
    require(
        lean_df.get("gate") == "scripts/run_rtl_m1_lean_witness_gate.sh",
        "no_forged_authority: Lean decode-faithfulness must name its gate",
    )
    require(
        lean_df.get("tactic") == "decide",
        "no_forged_authority: Lean decode-faithfulness must use kernel decide",
    )

    trace_sources = claim.get("trace_sources", [])
    gate_scripts = claim.get("gate_scripts", [])
    for source in ("formal/m1_top_refinement.py", "scripts/check_rtl_top_m1_witness.py"):
        require(source in trace_sources, f"no_forged_authority: trace_sources must include {source}")
    for gate in (
        "scripts/run_rtl_top_m1_witness_gate.sh",
        "scripts/check_rtl_top_m1_witness.py",
        "scripts/run_rtl_m1_lean_witness_gate.sh",
        "scripts/gen_m1_witness_lean.py",
    ):
        require(gate in gate_scripts, f"no_forged_authority: gate_scripts must include {gate}")
    known_gaps = " ".join(claim.get("known_gaps", []))
    require(
        "scripts/check_rtl_top_m1_witness.py" in known_gaps,
        "no_forged_authority: known gap must record the offline witness re-check",
    )
    require(
        "scripts/run_rtl_m1_lean_witness_gate.sh" in known_gaps,
        "no_forged_authority: known gap must record the Lean decode-faithfulness proof",
    )


def check_m7_typed_trace_contract(claim: dict) -> None:
    claim_id = claim.get("id")
    if claim_id not in {"scheduler_single_location", "no_lost_wakeups"}:
        return
    trace_sources = claim.get("trace_sources")
    require(isinstance(trace_sources, list), f"{claim_id}: trace_sources must be a list")
    for source in (
        "scripts/check_rtl_m7_typed_commit_trace.py",
        "scripts/test_rtl_m7_typed_commit_checker.py",
    ):
        require(source in trace_sources, f"{claim_id}: missing M7 typed trace source {source}")

    trace_markers = claim.get("trace_markers")
    require(isinstance(trace_markers, list), f"{claim_id}: trace_markers must be a list")
    for marker in (
        'TTRACE_M7 {\\"record\\":\\"m7_sched_commit\\"',
        'TTRACE_M7_STATE {\\"record\\":\\"m7_state_projection\\"',
        "rtl m7 typed commit trace ok",
    ):
        require(marker in trace_markers, f"{claim_id}: missing M7 typed trace marker {marker}")

    for source in (
        "scripts/check_rtl_m7_witness.py",
        "scripts/run_rtl_m7_witness_gate.sh",
        "scripts/gen_m7_witness_lean.py",
        "scripts/run_rtl_m7_lean_witness_gate.sh",
    ):
        require(source in trace_sources, f"{claim_id}: missing M7 witness source {source}")

    gates = claim.get("gate_scripts")
    require(isinstance(gates, list), f"{claim_id}: gate_scripts must be a list")
    for gate in (
        "scripts/check_rtl_m7_typed_commit_trace.py",
        "scripts/test_rtl_m7_typed_commit_checker.py",
        "scripts/check_rtl_m7_witness.py",
        "scripts/run_rtl_m7_witness_gate.sh",
        "scripts/test_rtl_m7_witness_checker.py",
        "scripts/gen_m7_witness_lean.py",
        "scripts/run_rtl_m7_lean_witness_gate.sh",
    ):
        require(gate in gates, f"{claim_id}: missing M7 typed trace gate {gate}")

    known_gaps = " ".join(claim.get("known_gaps", []))
    require(
        "typed transition traces" not in known_gaps,
        f"{claim_id}: known gap still claims M7 typed transition traces are missing",
    )
    require(
        "RTL-to-Lean refinement" in known_gaps or "multi-source event-router refinement" in known_gaps,
        f"{claim_id}: known gap must keep the remaining refinement gap explicit",
    )
    require(
        "scripts/check_rtl_m7_witness.py" in known_gaps,
        f"{claim_id}: known gap must record the offline M7 witness re-check",
    )
    require(
        "scripts/run_rtl_m7_lean_witness_gate.sh" in known_gaps,
        f"{claim_id}: known gap must record the M7 Lean decode-faithfulness proof",
    )

    for witness_file in (
        "scripts/check_rtl_m7_witness.py",
        "scripts/test_rtl_m7_witness_checker.py",
        "scripts/run_rtl_m7_witness_gate.sh",
        "scripts/gen_m7_witness_lean.py",
        "scripts/run_rtl_m7_lean_witness_gate.sh",
    ):
        require((ROOT / witness_file).exists(), f"{claim_id}: missing M7 witness artifact {witness_file}")

    proof_gate_text = check_file(RTL_PROOF_GATES, "RTL proof gate")
    require(
        "scripts/check_rtl_m7_typed_commit_trace.py" in proof_gate_text,
        "RTL proof gate must run the M7 typed trace checker",
    )
    require(
        "scripts/test_rtl_m7_typed_commit_checker.py" in proof_gate_text,
        "RTL proof gate must run the M7 typed trace checker self-test",
    )
    require(
        "scripts/check_rtl_m7_witness.py" in proof_gate_text,
        "RTL proof gate must consume the M7 scheduler witness",
    )
    require(
        "scripts/test_rtl_m7_witness_checker.py" in proof_gate_text,
        "RTL proof gate must run the M7 witness checker self-test",
    )
    require(
        "scripts/run_rtl_m7_lean_witness_gate.sh" in proof_gate_text,
        "RTL proof gate must run the M7 Lean decode-faithfulness gate",
    )


def check_m4_typed_trace_contract(claim: dict) -> None:
    if claim.get("id") != "vma_memory_safety":
        return
    trace_sources = claim.get("trace_sources", [])
    gate_scripts = claim.get("gate_scripts", [])
    m4_artifacts = (
        "scripts/check_rtl_m4_typed_commit_trace.py",
        "scripts/test_rtl_m4_typed_commit_checker.py",
        "scripts/check_rtl_m4_witness.py",
        "scripts/run_rtl_m4_witness_gate.sh",
        "scripts/test_rtl_m4_witness_checker.py",
        "scripts/gen_m4_witness_lean.py",
        "scripts/run_rtl_m4_lean_witness_gate.sh",
    )
    for name in m4_artifacts:
        require((ROOT / name).exists(), f"vma_memory_safety: missing M4 artifact {name}")
        require(name in gate_scripts, f"vma_memory_safety: missing M4 gate {name}")
    for name in (
        "scripts/check_rtl_m4_typed_commit_trace.py",
        "scripts/check_rtl_m4_witness.py",
        "scripts/run_rtl_m4_lean_witness_gate.sh",
    ):
        require(name in trace_sources, f"vma_memory_safety: missing M4 trace source {name}")
    markers = claim.get("trace_markers", [])
    require(
        'TTRACE_M4 {\\"record\\":\\"m4_vma_commit\\"' in markers,
        "vma_memory_safety: missing M4 typed commit trace marker",
    )
    require("rtl m4 typed commit trace ok" in markers, "vma_memory_safety: missing M4 typed trace pass marker")
    known_gaps = " ".join(claim.get("known_gaps", []))
    require(
        "typed transition traces" not in known_gaps,
        "vma_memory_safety: known gap still claims M4 typed transition traces are missing",
    )
    require(
        "scripts/check_rtl_m4_typed_commit_trace.py" in known_gaps,
        "vma_memory_safety: known gap must record the M4 typed trace contract",
    )
    require(
        "scripts/run_rtl_m4_lean_witness_gate.sh" in known_gaps,
        "vma_memory_safety: known gap must record the M4 Lean decode-faithfulness proof",
    )
    proof_gate_text = check_file(RTL_PROOF_GATES, "RTL proof gate")
    for name in (
        "scripts/check_rtl_m4_typed_commit_trace.py",
        "scripts/test_rtl_m4_typed_commit_checker.py",
        "scripts/check_rtl_m4_witness.py",
        "scripts/test_rtl_m4_witness_checker.py",
        "scripts/run_rtl_m4_lean_witness_gate.sh",
    ):
        require(name in proof_gate_text, f"RTL proof gate must run {name}")


def check_m5_typed_trace_contract(claim: dict) -> None:
    if claim.get("id") != "dma_confined":
        return
    trace_sources = claim.get("trace_sources", [])
    gate_scripts = claim.get("gate_scripts", [])
    m5_artifacts = (
        "scripts/check_rtl_m5_typed_commit_trace.py",
        "scripts/test_rtl_m5_typed_commit_checker.py",
        "scripts/check_rtl_m5_witness.py",
        "scripts/run_rtl_m5_witness_gate.sh",
        "scripts/test_rtl_m5_witness_checker.py",
        "scripts/gen_m5_witness_lean.py",
        "scripts/run_rtl_m5_lean_witness_gate.sh",
    )
    for name in m5_artifacts:
        require((ROOT / name).exists(), f"dma_confined: missing M5 artifact {name}")
        require(name in gate_scripts, f"dma_confined: missing M5 gate {name}")
    for name in (
        "scripts/check_rtl_m5_typed_commit_trace.py",
        "scripts/check_rtl_m5_witness.py",
        "scripts/run_rtl_m5_lean_witness_gate.sh",
    ):
        require(name in trace_sources, f"dma_confined: missing M5 trace source {name}")
    markers = claim.get("trace_markers", [])
    require(
        'TTRACE_M5 {\\"record\\":\\"m5_dma_commit\\"' in markers,
        "dma_confined: missing M5 typed commit trace marker",
    )
    require("rtl m5 typed commit trace ok" in markers, "dma_confined: missing M5 typed trace pass marker")
    known_gaps = " ".join(claim.get("known_gaps", []))
    require(
        "typed transition traces" not in known_gaps,
        "dma_confined: known gap still claims M5 typed transition traces are missing",
    )
    require(
        "scripts/run_rtl_m5_lean_witness_gate.sh" in known_gaps,
        "dma_confined: known gap must record the M5 Lean decode-faithfulness proof",
    )
    proof_gate_text = check_file(RTL_PROOF_GATES, "RTL proof gate")
    for name in (
        "scripts/check_rtl_m5_typed_commit_trace.py",
        "scripts/test_rtl_m5_typed_commit_checker.py",
        "scripts/check_rtl_m5_witness.py",
        "scripts/test_rtl_m5_witness_checker.py",
        "scripts/run_rtl_m5_lean_witness_gate.sh",
    ):
        require(name in proof_gate_text, f"RTL proof gate must run {name}")


def check_engine_typed_trace_contract(
    claim: dict,
    claim_id: str,
    prefix: str,
    commit_marker: str,
    pass_marker: str,
) -> None:
    """Shared enforcement that a milestone slice carries the full typed-trace,
    witness, and Lean decode-faithfulness apparatus and is wired into the gate."""
    if claim.get("id") != claim_id:
        return
    trace_sources = claim.get("trace_sources", [])
    gate_scripts = claim.get("gate_scripts", [])
    artifacts = (
        f"scripts/check_rtl_{prefix}_typed_commit_trace.py",
        f"scripts/test_rtl_{prefix}_typed_commit_checker.py",
        f"scripts/check_rtl_{prefix}_witness.py",
        f"scripts/run_rtl_{prefix}_witness_gate.sh",
        f"scripts/test_rtl_{prefix}_witness_checker.py",
        f"scripts/gen_{prefix}_witness_lean.py",
        f"scripts/run_rtl_{prefix}_lean_witness_gate.sh",
    )
    for name in artifacts:
        require((ROOT / name).exists(), f"{claim_id}: missing {prefix} artifact {name}")
        require(name in gate_scripts, f"{claim_id}: missing {prefix} gate {name}")
    for name in (
        f"scripts/check_rtl_{prefix}_typed_commit_trace.py",
        f"scripts/check_rtl_{prefix}_witness.py",
        f"scripts/run_rtl_{prefix}_lean_witness_gate.sh",
    ):
        require(name in trace_sources, f"{claim_id}: missing {prefix} trace source {name}")
    markers = claim.get("trace_markers", [])
    require(commit_marker in markers, f"{claim_id}: missing {prefix} typed commit trace marker")
    require(pass_marker in markers, f"{claim_id}: missing {prefix} typed trace pass marker")
    known_gaps = " ".join(claim.get("known_gaps", []))
    require(
        "typed transition traces" not in known_gaps,
        f"{claim_id}: known gap still claims {prefix} typed transition traces are missing",
    )
    require(
        f"scripts/run_rtl_{prefix}_lean_witness_gate.sh" in known_gaps,
        f"{claim_id}: known gap must record the {prefix} Lean decode-faithfulness proof",
    )
    proof_gate_text = check_file(RTL_PROOF_GATES, "RTL proof gate")
    for name in (
        f"scripts/check_rtl_{prefix}_typed_commit_trace.py",
        f"scripts/test_rtl_{prefix}_typed_commit_checker.py",
        f"scripts/check_rtl_{prefix}_witness.py",
        f"scripts/test_rtl_{prefix}_witness_checker.py",
        f"scripts/run_rtl_{prefix}_lean_witness_gate.sh",
    ):
        require(name in proof_gate_text, f"RTL proof gate must run {name}")


def check_file(path: Path, label: str) -> str:
    require(path.exists(), f"missing {label} {path}")
    require(path.stat().st_size > 0, f"empty {label} {path}")
    return read_text(path)


def check_claim(claim: dict) -> None:
    claim_id = claim.get("id")
    require(isinstance(claim_id, str) and claim_id, "claim missing id")
    require(isinstance(claim.get("claim"), str) and claim["claim"], f"{claim_id}: missing claim text")
    check_m1_top_level_contract(claim)
    check_m7_typed_trace_contract(claim)
    check_m4_typed_trace_contract(claim)
    check_m5_typed_trace_contract(claim)
    check_engine_typed_trace_contract(
        claim,
        "gate_fault_delivery_safety",
        "m2",
        'TTRACE_M2 {\\"record\\":\\"m2_gate_commit\\"',
        "rtl m2 typed commit trace ok",
    )

    trust = claim.get("trust_level")
    require(trust in ALLOWED_TRUST_LEVELS, f"{claim_id}: invalid trust level {trust}")
    require(claim.get("known_gaps"), f"{claim_id}: known_gaps must be explicit")
    require(claim.get("assumptions"), f"{claim_id}: assumptions must be explicit")

    lean_theorems = claim.get("lean_theorems")
    require(isinstance(lean_theorems, list) and lean_theorems, f"{claim_id}: missing Lean theorem links")
    for item in lean_theorems:
        path = root_path(item["file"])
        check_file(path, f"{claim_id} Lean file")
        theorem = item["name"]
        artifact_level = item.get("artifact_level")
        require(
            artifact_level in ALLOWED_ARTIFACT_LEVELS,
            f"{claim_id}: {theorem} has invalid artifact_level {artifact_level}",
        )
        if item["file"] == "formal/FormalTheoremsModel.lean":
            require(
                artifact_level == "coverage",
                f"{claim_id}: FormalTheoremsModel.lean entries must be coverage artifacts",
            )
        require(theorem_exists(path, theorem), f"{claim_id}: missing theorem {theorem} in {item['file']}")

    rtl_text = ""
    modules = claim.get("rtl_modules")
    require(isinstance(modules, list) and modules, f"{claim_id}: missing RTL modules")
    for item in modules:
        path = root_path(item["file"])
        rtl_text += "\n" + check_file(path, f"{claim_id} RTL file")
        require(module_exists(path, item["module"]), f"{claim_id}: missing module {item['module']} in {item['file']}")

    assertion_text = ""
    assertion_files = claim.get("assertion_files")
    require(isinstance(assertion_files, list) and assertion_files, f"{claim_id}: missing assertion files")
    for name in assertion_files:
        assertion_text += "\n" + check_file(root_path(name), f"{claim_id} assertion file")

    combined_signal_text = rtl_text + "\n" + assertion_text
    for signal in claim.get("rtl_witness_signals", []):
        require(signal in combined_signal_text, f"{claim_id}: missing RTL/assertion witness {signal}")

    trace_text = ""
    trace_sources = claim.get("trace_sources")
    require(isinstance(trace_sources, list) and trace_sources, f"{claim_id}: missing trace sources")
    for name in trace_sources:
        trace_text += "\n" + check_file(root_path(name), f"{claim_id} trace source")
    for marker in claim.get("trace_markers", []):
        require(marker in trace_text, f"{claim_id}: missing trace marker {marker}")

    gates = claim.get("gate_scripts")
    require(isinstance(gates, list) and gates, f"{claim_id}: missing gate scripts")
    for name in gates:
        path = root_path(name)
        check_file(path, f"{claim_id} gate script")
        require(os.access(path, os.X_OK), f"{claim_id}: gate script is not executable {name}")


def main() -> None:
    manifest = json.loads(check_file(MANIFEST, "theorem/RTL coupling manifest"))
    require(manifest.get("name") == "lnp64_theorem_rtl_coupling", "unexpected coupling manifest name")
    require(root_path(manifest["roadmap"]).exists(), f"missing roadmap {manifest['roadmap']}")

    trust_levels = manifest.get("trust_levels")
    require(isinstance(trust_levels, dict), "trust_levels must be an object")
    for level in ALLOWED_TRUST_LEVELS:
        require(level in trust_levels, f"trust_levels omits {level}")

    claims = manifest.get("claims")
    require(isinstance(claims, list) and claims, "coupling manifest must list claims")
    index_text = check_file(INDEX, "theorem/RTL coupling index")
    seen: set[str] = set()
    required_claims = {
        "no_forged_authority",
        "revocation_generation_safety",
        "domain_containment",
        "dma_confined",
        "scheduler_single_location",
        "no_lost_wakeups",
        "servicelets_terminate_contained",
        "faults_terminal_progress",
    }
    for claim in claims:
        claim_id = claim.get("id")
        require(claim_id not in seen, f"duplicate coupling claim {claim_id}")
        seen.add(claim_id)
        check_claim(claim)
        require(f"`{claim_id}`" in index_text, f"coupling index omits claim {claim_id}")
        require(claim["trust_level"] in index_text, f"coupling index omits trust level for {claim_id}")

    missing = sorted(required_claims - seen)
    require(not missing, f"coupling manifest omits required claims: {', '.join(missing)}")
    print("theorem/RTL coupling manifest ok")


if __name__ == "__main__":
    main()

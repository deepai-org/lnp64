#!/usr/bin/env python3
"""Check formal proof obligation coverage against Lean theorem artifacts."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path(os.environ.get("LNP64_PROOF_MANIFEST", str(ROOT / "formal/proof_obligations_manifest.json")))
ROADMAP = Path(os.environ.get("LNP64_ROADMAP", str(ROOT / "formal_rtl_codesign_roadmap.md")))
FORMAL_THEOREMS = Path(os.environ.get("LNP64_FORMAL_THEOREMS", str(ROOT / "formal_theorems.md")))
PROOF_GATE = Path(os.environ.get("LNP64_PROOF_GATE", str(ROOT / "scripts/run_rtl_proof_gates.sh")))
EXPECTED_IDS = ["S0"] + [f"A{i}" for i in range(1, 11)] + ["FT"]
FORBIDDEN_LEAN_PLACEHOLDER = re.compile(r"(^|[^A-Za-z0-9_])(axiom|sorry|admit)([^A-Za-z0-9_]|$)")
ALLOWED_ARTIFACT_LEVELS = {
    "coverage",
    "bounded_witness",
    "transition_invariant",
    "refinement",
}
ALLOWED_TRUST_LEVELS = {"T0", "T1", "T2", "T3", "T4", "T5"}
REQUIRED_BRIDGE_FIELDS = [
    "assumptions",
    "rtl_modules",
    "rtl_witness_signals",
    "trace_evidence",
    "trust_level",
    "known_gaps",
]
REQUIRED_S0_THEOREMS = [
    "s0_reset_produces_valid_initial_state_or_measured_fault",
    "s0_every_live_thread_has_exactly_one_scheduler_location",
    "s0_every_accepted_command_has_at_most_one_terminal_response_event_or_fault",
    "s0_every_accepted_command_has_terminal_path_under_fairness",
    "s0_stubs_do_not_create_authority",
    "s0_unsupported_operations_fail_closed",
    "s0_parked_threads_name_valid_wake_timeout_cancel_fault_or_completion_source",
    "s0_software_visible_records_contain_no_raw_interrupt_or_physical_address_authority",
]
REQUIRED_THEOREMS_BY_ID = {
    "S0": REQUIRED_S0_THEOREMS,
    "A1": [
        "s0_state_core_well_formed",
        "s0_no_forged_fdrs",
        "s0_generation_checks_hold",
        "s0_domain_parent_validity",
    ],
    "A2": [
        "m1_no_forged_fdr",
        "m1_no_authority_amplification",
        "m1_cap_send_preserves_narrowing",
        "m1_cap_recv_installs_sent_cap",
        "m1_cap_revoke_invalidates_generation",
        "m1_revoked_authority_cannot_start_new_work",
        "m1_stale_generation_rejected",
    ],
    "A3": [
        "m7_exactly_one_scheduler_location",
        "m7_no_lost_wakeup",
        "m7_timer_wait_parked",
        "m7_timer_expiry_wakes_thread",
        "m7_wake_generation_matches",
        "m7_domain_budget_eligible",
    ],
    "A4": [
        "m15_queue_rights_allow_push",
        "m15_queue_overflow_explicit",
        "m15_gate_continuation_unique",
        "m15_event_source_generation_safe",
    ],
    "A5": [
        "m4_no_invalid_memory_access",
        "m4_wx_nx_guard_enforced",
        "m5_dma_confined_to_capability_domain",
        "m5_pin_completes_with_authority",
        "m5_unpin_clears_pinned_state",
    ],
    "A6": [
        "m9_termination_by_construction",
        "m9_no_authority_creation",
        "m9_no_arbitrary_memory_access",
        "m9_network_action_contained",
    ],
    "A7": [
        "m14_child_rights_subset_parent",
        "m14_child_budget_within_parent",
        "m14_frozen_domain_cannot_dispatch",
        "m14_destroyed_domain_cannot_dispatch",
        "m14_usage_rolls_up",
        "m14_policy_fail_closed",
        "m14_policy_cannot_be_bypassed_by_another_engine",
    ],
    "A8": [
        "m2_continuation_unique",
        "m2_stale_continuation_rejected",
        "m2_fault_delivery_gate_entered",
        "m2_signal_compatibility_cannot_create_authority_or_bypass_masks",
    ],
    "A9": [
        "m7_locked_atomic_single_copy",
        "m4_no_access_after_unmap_or_revoke_generation_mismatch",
        "m5_dma_confined_to_capability_domain",
        "m4_cache_tlb_quiescent_before_authority_reuse",
    ],
    "A10": [
        "m10_adversarial_inputs_cannot_hang_owner_or_create_authority",
        "m10_bounded_local_fault_reaches_terminal_path",
        "m10_watchdog_reset_preserves_unrelated_domains",
        "m10_realtime_work_has_bounded_arbitration_progress",
    ],
    "FT": [
        "ft_formal_model_scope",
        "ft_proof_fault_assumptions_explicit",
        "ft_security_theorem_spine",
        "ft_proof_priority_order",
        "ft_global_state_validity",
        "ft_capability_non_forgeability",
        "ft_no_authority_amplification",
        "ft_revocation_soundness",
        "ft_generation_safety",
        "ft_resource_domain_containment",
        "ft_scheduler_safety",
        "ft_realtime_boundedness",
        "ft_default_operating_envelope",
        "ft_no_lost_wakeups",
        "ft_object_profile_refinement",
        "ft_namespace_dispatch_containment",
        "ft_typed_control_envelope_safety",
        "ft_service_domain_boundary_soundness",
        "ft_vma_memory_safety",
        "ft_memory_visibility_contract",
        "ft_wx_executable_provenance",
        "ft_heap_allocation_safety",
        "ft_dma_isolation",
        "ft_raw_interrupt_non_exposure",
        "ft_network_capability_containment",
        "ft_classifier_servicelet_safety",
        "ft_event_gate_fault_delivery_safety",
        "ft_gate_continuation_safety",
        "ft_checkpoint_hook_safety",
        "ft_commit_abort_atomicity",
        "ft_clone_fork_profile_safety",
        "ft_storage_filesystem_durability",
        "ft_exec_plan_commit_soundness",
        "ft_boot_measurement_attestation_integrity",
        "ft_assurance_profile_policy_soundness",
        "ft_owner_sovereignty_open_assurance",
        "ft_ras_fault_containment",
        "ft_telemetry_trace_counter_non_interference",
        "ft_tamper_evident_audit_integrity",
        "ft_posix_profile_compatibility_refinement",
        "ft_paravirtual_personality_containment",
        "ft_tenant_strict_confidentiality",
        "ft_controlled_debug_forensics_non_bypass",
        "ft_cross_domain_mls_noninterference",
        "ft_mission_assurance_continuity",
        "ft_global_progress_bounded_faults",
        "ft_adversarial_input_containment",
        "ft_refinement_targets",
    ],
}
FORMAL_THEOREM_SECTION_REQUIREMENTS = {
    "0. Formal Model Scope": "ft_formal_model_scope",
    "0.1 Proof and Fault Model Assumptions": "ft_proof_fault_assumptions_explicit",
    "0.2 Security Theorem Spine": "ft_security_theorem_spine",
    "0.3 Proof Priority Order": "ft_proof_priority_order",
    "1. Global State Validity": "ft_global_state_validity",
    "2. Capability Non-Forgeability": "ft_capability_non_forgeability",
    "3. No Authority Amplification": "ft_no_authority_amplification",
    "4. Revocation Soundness": "ft_revocation_soundness",
    "5. Generation Safety": "ft_generation_safety",
    "6. Resource Domain Containment": "ft_resource_domain_containment",
    "7. Scheduler Safety": "ft_scheduler_safety",
    "7.1 Realtime Boundedness": "ft_realtime_boundedness",
    "7.2 Default Operating Envelope": "ft_default_operating_envelope",
    "8. No Lost Wakeups": "ft_no_lost_wakeups",
    "9. Object Profile Refinement": "ft_object_profile_refinement",
    "10. Namespace Dispatch Containment": "ft_namespace_dispatch_containment",
    "11. Typed Control Envelope Safety": "ft_typed_control_envelope_safety",
    "11.1 Service Domain Boundary Soundness": "ft_service_domain_boundary_soundness",
    "12. VMA and Memory Safety": "ft_vma_memory_safety",
    "12.1 Memory Visibility Contract": "ft_memory_visibility_contract",
    "13. W^X and Executable Provenance": "ft_wx_executable_provenance",
    "14. Heap Allocation Safety": "ft_heap_allocation_safety",
    "15. DMA Isolation": "ft_dma_isolation",
    "16. Raw Interrupt Non-Exposure": "ft_raw_interrupt_non_exposure",
    "17. Network Capability Containment": "ft_network_capability_containment",
    "18. Classifier and Servicelet Safety": "ft_classifier_servicelet_safety",
    "19. Event, Gate, and Fault Delivery Safety": "ft_event_gate_fault_delivery_safety",
    "20. Gate/Continuation Safety": "ft_gate_continuation_safety",
    "21. Checkpoint Hook Safety": "ft_checkpoint_hook_safety",
    "22. Commit/Abort Atomicity": "ft_commit_abort_atomicity",
    "23. Clone/Fork Profile Safety": "ft_clone_fork_profile_safety",
    "24. Storage and Filesystem-Service Durability Contract": "ft_storage_filesystem_durability",
    "25. Exec-Plan Commit Soundness": "ft_exec_plan_commit_soundness",
    "26. Boot Measurement and Attestation Integrity": "ft_boot_measurement_attestation_integrity",
    "26.1 Assurance Profile and Policy Enforcement Soundness": "ft_assurance_profile_policy_soundness",
    "26.2 Owner Sovereignty and Open Assurance": "ft_owner_sovereignty_open_assurance",
    "27. RAS Fault Containment": "ft_ras_fault_containment",
    "28. Telemetry, Trace, and Counter Non-Interference": "ft_telemetry_trace_counter_non_interference",
    "28.1 Tamper-Evident Audit Integrity": "ft_tamper_evident_audit_integrity",
    "29. POSIX/Profile Compatibility Refinement": "ft_posix_profile_compatibility_refinement",
    "30. Paravirtual Personality Containment": "ft_paravirtual_personality_containment",
    "31. Tenant-Strict Confidentiality and No Unauthorized Observation": "ft_tenant_strict_confidentiality",
    "31.1 Controlled Debug and Forensics Non-Bypass": "ft_controlled_debug_forensics_non_bypass",
    "31.2 Cross-Domain MLS Noninterference": "ft_cross_domain_mls_noninterference",
    "31.3 Mission Assurance Continuity": "ft_mission_assurance_continuity",
    "32. Global Progress Under Bounded Faults": "ft_global_progress_bounded_faults",
    "33. Adversarial Input Containment": "ft_adversarial_input_containment",
    "34. Refinement Targets": "ft_refinement_targets",
}


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def display_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def theorem_exists(source: str, theorem: str) -> bool:
    return re.search(rf"(?m)^theorem\s+{re.escape(theorem)}\b", source) is not None


def severe_goals_from_roadmap(roadmap_text: str) -> set[str]:
    goals = set(re.findall(r"`(SG-[A-Z]+)`", roadmap_text))
    required = {"SG-AUTH", "SG-ISO", "SG-SCHED", "SG-WAKE", "SG-MEM", "SG-PROGRESS", "SG-RT", "SG-EVIDENCE"}
    require(required <= goals, f"roadmap omits severe proof goals: {sorted(required - goals)}")
    return goals


def check_nonempty_string_list(value: object, label: str) -> None:
    require(isinstance(value, list) and value, f"{label} must be a non-empty list")
    for item in value:
        require(isinstance(item, str) and item, f"{label} contains an empty/non-string item")


def check_bridge_metadata(group: dict, obligation_id: str, severe_goals: set[str]) -> None:
    goals = group.get("severe_goals")
    check_nonempty_string_list(goals, f"{obligation_id}: severe_goals")
    unknown = sorted(set(goals) - severe_goals)
    require(not unknown, f"{obligation_id}: unknown severe goals: {', '.join(unknown)}")

    artifact_level = group.get("artifact_level")
    require(
        artifact_level in ALLOWED_ARTIFACT_LEVELS,
        f"{obligation_id}: invalid artifact_level {artifact_level}",
    )

    bridge = group.get("bridge")
    require(isinstance(bridge, dict), f"{obligation_id}: missing bridge metadata")
    for field in REQUIRED_BRIDGE_FIELDS:
        require(field in bridge, f"{obligation_id}: bridge missing {field}")
    for field in ["assumptions", "rtl_modules", "rtl_witness_signals", "trace_evidence", "known_gaps"]:
        check_nonempty_string_list(bridge[field], f"{obligation_id}: bridge.{field}")
    trust_level = bridge["trust_level"]
    require(trust_level in ALLOWED_TRUST_LEVELS, f"{obligation_id}: invalid bridge trust_level {trust_level}")


def check_theorem_group(group: dict, obligation_id: str, proof_gate_text: str) -> None:
    lean_file = ROOT / group["lean_file"]
    require(lean_file.exists(), f"{obligation_id}: missing Lean file {group['lean_file']}")
    require(
        group["lean_file"] in proof_gate_text,
        f"{obligation_id}: Lean file {group['lean_file']} is not checked by {display_path(PROOF_GATE)}",
    )
    source = read_text(lean_file)
    for line_number, line in enumerate(source.splitlines(), start=1):
        require(
            FORBIDDEN_LEAN_PLACEHOLDER.search(line) is None,
            f"{obligation_id}: forbidden Lean placeholder in {group['lean_file']}:{line_number}",
        )
    theorems = group.get("theorems")
    require(isinstance(theorems, list) and theorems, f"{obligation_id}: no theorem names")
    for theorem in theorems:
        require(isinstance(theorem, str) and theorem, f"{obligation_id}: empty theorem name")
        require(theorem_exists(source, theorem), f"{obligation_id}: missing theorem {theorem} in {group['lean_file']}")


def check_required_theorems(obligation: dict) -> None:
    obligation_id = obligation["id"]
    listed = set(obligation.get("theorems", []))
    for extra in obligation.get("additional_lean_files", []):
        listed.update(extra.get("theorems", []))
    for theorem in REQUIRED_THEOREMS_BY_ID.get(obligation_id, []):
        require(theorem in listed, f"{obligation_id}: missing required roadmap theorem {theorem}")


def check_formal_theorem_sections(obligation: dict) -> None:
    if obligation["id"] != "FT":
        return

    source = obligation.get("source")
    require(source == "formal_theorems.md", "FT: source must be formal_theorems.md")
    formal_theorems_text = read_text(FORMAL_THEOREMS)
    listed = set(obligation.get("theorems", []))
    for section, theorem in FORMAL_THEOREM_SECTION_REQUIREMENTS.items():
        require(f"## {section}" in formal_theorems_text, f"FT: formal_theorems.md omits section {section}")
        require(theorem in listed, f"FT: missing theorem artifact for formal_theorems.md section {section}")


def main() -> None:
    require(MANIFEST.exists(), f"missing proof manifest {MANIFEST}")
    require(ROADMAP.exists(), f"missing roadmap {ROADMAP}")
    require(FORMAL_THEOREMS.exists(), f"missing formal theorem roadmap {FORMAL_THEOREMS}")
    require(PROOF_GATE.exists(), f"missing proof gate {PROOF_GATE}")

    manifest = json.loads(read_text(MANIFEST))
    require(manifest.get("track") == "Track A: Formal Model", "manifest must name Track A")
    proof_gate_text = read_text(PROOF_GATE)

    obligations = manifest.get("obligations")
    require(isinstance(obligations, list), "manifest obligations must be a list")
    ids = [obligation.get("id") for obligation in obligations]
    require(ids == EXPECTED_IDS, f"proof obligation ids must be {EXPECTED_IDS}")

    raw_roadmap_text = read_text(ROADMAP)
    severe_goals = severe_goals_from_roadmap(raw_roadmap_text)
    roadmap_text = raw_roadmap_text.replace("`", "")
    formal_theorems_text = read_text(FORMAL_THEOREMS).replace("`", "")
    for obligation in obligations:
        obligation_id = obligation["id"]
        title = obligation.get("title")
        require(isinstance(title, str) and title, f"{obligation_id}: missing title")
        title_source = formal_theorems_text if obligation_id == "FT" else roadmap_text
        require(title in title_source, f"{obligation_id}: title not present in roadmap source: {title}")

        check_bridge_metadata(obligation, obligation_id, severe_goals)
        check_required_theorems(obligation)
        check_formal_theorem_sections(obligation)
        check_theorem_group(obligation, obligation_id, proof_gate_text)
        for extra in obligation.get("additional_lean_files", []):
            check_theorem_group(extra, obligation_id, proof_gate_text)

    print("formal proof manifest ok")


if __name__ == "__main__":
    main()

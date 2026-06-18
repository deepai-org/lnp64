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
PROOF_GATE = Path(os.environ.get("LNP64_PROOF_GATE", str(ROOT / "scripts/run_rtl_proof_gates.sh")))
EXPECTED_IDS = ["S0"] + [f"A{i}" for i in range(1, 11)]
FORBIDDEN_LEAN_PLACEHOLDER = re.compile(r"(^|[^A-Za-z0-9_])(axiom|sorry|admit)([^A-Za-z0-9_]|$)")
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


def main() -> None:
    require(MANIFEST.exists(), f"missing proof manifest {MANIFEST}")
    require(ROADMAP.exists(), f"missing roadmap {ROADMAP}")
    require(PROOF_GATE.exists(), f"missing proof gate {PROOF_GATE}")

    manifest = json.loads(read_text(MANIFEST))
    require(manifest.get("track") == "Track A: Formal Model", "manifest must name Track A")
    proof_gate_text = read_text(PROOF_GATE)

    obligations = manifest.get("obligations")
    require(isinstance(obligations, list), "manifest obligations must be a list")
    ids = [obligation.get("id") for obligation in obligations]
    require(ids == EXPECTED_IDS, f"proof obligation ids must be {EXPECTED_IDS}")

    roadmap_text = read_text(ROADMAP).replace("`", "")
    for obligation in obligations:
        obligation_id = obligation["id"]
        title = obligation.get("title")
        require(isinstance(title, str) and title, f"{obligation_id}: missing title")
        require(title in roadmap_text, f"{obligation_id}: title not present in roadmap: {title}")

        check_required_theorems(obligation)
        check_theorem_group(obligation, obligation_id, proof_gate_text)
        for extra in obligation.get("additional_lean_files", []):
            check_theorem_group(extra, obligation_id, proof_gate_text)

    print("formal proof manifest ok")


if __name__ == "__main__":
    main()

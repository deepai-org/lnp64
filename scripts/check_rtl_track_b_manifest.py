#!/usr/bin/env python3
"""Check Track B RTL block coverage against concrete gates and RTL artifacts."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path(os.environ.get("LNP64_TRACK_B_MANIFEST", str(ROOT / "rtl/track_b_blocks_manifest.json")))
ROADMAP = Path(os.environ.get("LNP64_ROADMAP", str(ROOT / "formal_rtl_codesign_roadmap.md")))
SYNTH_SMOKE = Path(os.environ.get("LNP64_SYNTH_SMOKE", str(ROOT / "scripts/run_rtl_synth_smoke.sh")))
PROOF_GATE = Path(os.environ.get("LNP64_PROOF_GATE", str(ROOT / "scripts/run_rtl_proof_gates.sh")))
EXPECTED_IDS = ["B0"] + [f"B{i}" for i in range(1, 15)]
ALLOWED_STATUS = {"bounded_smoke"}
REQUIRED_B0_MODULES = [
    "lnp64_top",
    "lnp64_reset_boot",
    "lnp64_clock_reset",
    "lnp64_core_tile",
    "lnp64_decode",
    "lnp64_issue_retire",
    "lnp64_thread_context",
    "lnp64_engine_router",
    "lnp64_completion_router",
    "lnp64_errno_writeback",
    "lnp64_scheduler",
    "lnp64_event_router",
    "lnp64_cap_engine",
    "lnp64_domain_engine",
    "lnp64_policy_engine",
    "lnp64_object_engine",
    "lnp64_gate_engine",
    "lnp64_process_engine",
    "lnp64_vma_engine",
    "lnp64_page_allocator",
    "lnp64_memory_fabric",
    "lnp64_metadata_broker",
    "lnp64_dma_fabric",
    "lnp64_service_boundary",
    "lnp64_futex_atomic",
    "lnp64_heap_engine",
    "lnp64_classifier_servicelet",
    "lnp64_fault_telemetry",
    "lnp64_watchdog",
    "lnp64_measurement_attestation",
    "lnp64_entropy_env",
    "lnp64_uart",
    "lnp64_storage_stub",
    "lnp64_eth_stub",
    "lnp64_pcie_stub",
    "lnp64_typed_control_validator",
    "lnp64_namespace_dispatch",
    "lnp64_stream_frontend",
    "lnp64_ddr_controller",
    "lnp64_sd_spi_flash",
    "lnp64_boot_image_storage",
]

REQUIRED_BLOCK_RUN_MARKERS = {
    "B0": [
        "LNP64-RTL-S0 PASS",
        "forced boot fault did not emit measured/audited boot fault",
        "raw physical interrupt/address/DMA/device authority became visible",
    ],
    "B1": [
        "LNP64_OP_ENV_GET",
        "LNP64_OP_UNSUPPORTED",
        "ENV_GET did not report expected S0 feature bits",
    ],
    "B2": [
        "LNP64_OP_LD",
        "LNP64_OP_ST",
        "SRAM LD/ST path did not roundtrip the ALU value",
    ],
    "B3": [
        "TRACE cap_dup",
        "TRACE stale_pull",
    ],
    "B4": [
        "TRACE await",
        "TRACE futex_wake",
        "TRACE timer_wait",
        "TRACE timer_expire",
        "TRACE child_budget",
        "TRACE freeze",
    ],
    "B5": [
        "TRACE counter",
        "TRACE queue_overflow",
        "TRACE event_emit",
    ],
    "B6": [
        "TRACE gate_call",
        "TRACE gate_return",
        "TRACE fault_delivery",
        "TRACE signal_compat",
    ],
    "B7": [
        "TRACE clone",
        "TRACE exit",
        "TRACE join",
        "TRACE exec_barrier",
    ],
    "B8": [
        "TRACE store_denied",
        "TRACE exec_fault",
        "TRACE guard_fault",
        "TRACE tlb_invalidate",
    ],
    "B9": [
        "TRACE dma_pin",
        "TRACE dma_copy",
        "TRACE dma_fill",
        "TRACE dma_unpin",
        "TRACE domain_isolation",
        "TRACE coherence_flush",
    ],
    "B10": [
        "TRACE envelope",
        "TRACE ns_dispatch",
        "TRACE service_cancel",
        "TRACE crash_completion",
    ],
    "B11": [
        "TRACE cmpxchg",
        "TRACE futex_wait",
        "TRACE futex_wake",
        "TRACE stale_futex",
    ],
    "B12": [
        "TRACE alloc",
        "TRACE free",
        "TRACE double_free",
        "TRACE guard_fault",
    ],
    "B13": [
        "TRACE verifier",
        "TRACE packet_steer",
        "TRACE ipc_steer",
        "TRACE budget_exhaust",
    ],
    "B14": [
        "TRACE ecc_corrected",
        "TRACE parity_poison",
        "TRACE watchdog_timeout",
        "TRACE quote_stub",
    ],
}


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def module_exists(module: str) -> bool:
    pattern = re.compile(rf"\bmodule\s+{re.escape(module)}\b")
    return any(pattern.search(read_text(path)) for path in ROOT.glob("rtl/**/*.sv"))


def filelist_entries(path: Path) -> list[str]:
    entries: list[str] = []
    for raw in read_text(path).splitlines():
        line = raw.strip()
        if line and not line.startswith("#"):
            entries.append(line)
    return entries


def assertion_for_top(top: str) -> str:
    if top == "lnp64_s0_tb":
        return "formal/rtl_assertions/lnp64_s0_assertions.sv"
    match = re.fullmatch(r"lnp64_m([0-9]+)_tb", top)
    require(match is not None, f"unexpected RTL top {top}")
    return f"formal/rtl_assertions/lnp64_m{match.group(1)}_assertions.sv"


def main() -> None:
    require(MANIFEST.exists(), f"missing Track B manifest {MANIFEST}")
    require(ROADMAP.exists(), f"missing roadmap {ROADMAP}")
    require(SYNTH_SMOKE.exists(), f"missing synthesis smoke gate {SYNTH_SMOKE}")
    require(PROOF_GATE.exists(), f"missing proof gate {PROOF_GATE}")

    manifest = json.loads(read_text(MANIFEST))
    require(manifest.get("track") == "Track B: RTL Skeleton And Blocks", "manifest must name Track B")

    blocks = manifest.get("blocks")
    require(isinstance(blocks, list), "manifest blocks must be a list")
    ids = [block.get("id") for block in blocks]
    require(ids == EXPECTED_IDS, f"Track B block ids must be {EXPECTED_IDS}")

    roadmap_text = read_text(ROADMAP).replace("`", "")
    synth_smoke_text = read_text(SYNTH_SMOKE)
    proof_gate_text = read_text(PROOF_GATE)
    require("scripts/check_rtl_s0_contract.py" in synth_smoke_text, "synthesis smoke gate must run the S0 contract checker")

    for block in blocks:
        block_id = block["id"]
        title = block.get("title")
        status = block.get("status")
        scripts = block.get("gate_scripts")
        filelists = block.get("filelists")
        tops = block.get("rtl_tops")
        modules = block.get("modules")
        models = block.get("models")
        proof_models = block.get("proof_models")

        require(isinstance(title, str) and title, f"{block_id}: missing title")
        require(f"### {block_id}. {title}" in roadmap_text, f"{block_id}: title not present in roadmap")
        require(status in ALLOWED_STATUS, f"{block_id}: invalid status {status}")
        require(isinstance(scripts, list) and scripts, f"{block_id}: no gate scripts")
        require(isinstance(filelists, list) and filelists, f"{block_id}: no filelists")
        require(isinstance(tops, list) and tops, f"{block_id}: no RTL tops")
        require(isinstance(modules, list) and modules, f"{block_id}: no modules")
        require(isinstance(models, list), f"{block_id}: models must be a list")
        require(isinstance(proof_models, list) and proof_models, f"{block_id}: no Lean proof models")
        if block_id == "B0":
            missing_b0_modules = sorted(set(REQUIRED_B0_MODULES) - set(modules))
            require(not missing_b0_modules, f"B0 manifest omits required skeleton modules: {', '.join(missing_b0_modules)}")

        script_texts: dict[str, str] = {}
        for script_name in scripts:
            script = ROOT / script_name
            require(script.exists(), f"{block_id}: missing gate script {script_name}")
            require(script.stat().st_size > 0, f"{block_id}: empty gate script {script_name}")
            script_texts[script_name] = read_text(script)

        filelist_entries_by_name: dict[str, list[str]] = {}
        for filelist_name in filelists:
            filelist = ROOT / filelist_name
            require(filelist.exists(), f"{block_id}: missing filelist {filelist_name}")
            require(filelist_name in synth_smoke_text, f"{block_id}: {filelist_name} is not in synthesis smoke gate")
            entries = filelist_entries(filelist)
            require(entries, f"{block_id}: empty filelist {filelist_name}")
            for entry in entries:
                require((ROOT / entry).exists(), f"{block_id}: missing filelist source {entry}")
            filelist_entries_by_name[filelist_name] = entries

        all_entries = {entry for entries in filelist_entries_by_name.values() for entry in entries}
        block_source_text = "\n".join(read_text(ROOT / entry) for entry in sorted(all_entries))
        for marker in REQUIRED_BLOCK_RUN_MARKERS[block_id]:
            require(marker in block_source_text, f"{block_id}: missing roadmap run marker {marker}")

        for top in tops:
            require(any(f"--top-module {top}" in text for text in script_texts.values()), f"{block_id}: no script builds top {top}")
            require(f"rtl/sim/{top}.sv" in all_entries, f"{block_id}: filelists do not include testbench {top}")
            require(assertion_for_top(top) in all_entries, f"{block_id}: filelists do not include assertions for {top}")

        for module in modules:
            require(module_exists(module), f"{block_id}: missing RTL module {module}")

        for model_name in models:
            model = ROOT / model_name
            require(model.exists(), f"{block_id}: missing executable model {model_name}")
            require(any(model_name in text for text in script_texts.values()), f"{block_id}: no gate script runs {model_name}")

        for proof_model_name in proof_models:
            proof_model = ROOT / proof_model_name
            require(proof_model.exists(), f"{block_id}: missing Lean proof model {proof_model_name}")
            require(proof_model_name in proof_gate_text, f"{block_id}: proof gate does not check {proof_model_name}")

    print("rtl Track B manifest ok")


if __name__ == "__main__":
    main()

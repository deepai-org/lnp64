#!/usr/bin/env python3
"""Check the roadmap S0 RTL shell/record contract against concrete sources."""

from __future__ import annotations

import os
import re
from pathlib import Path


ROOT = Path(os.environ.get("LNP64_S0_CONTRACT_ROOT", str(Path(__file__).resolve().parents[1])))
FILELIST = ROOT / "tests/rtl/s0_filelist.f"
PKG = ROOT / "rtl/include/lnp64_pkg.sv"
S0_GATE = ROOT / "scripts/run_rtl_s0.sh"


REQUIRED_ACCEPTANCE_MARKERS = [
    "UART boot/status byte was not observed",
    "raw authority path was visible after boot",
    "forced boot fault created a stable boot state",
    "forced boot fault released the core",
    "forced boot fault used the wrong canonical fault code",
    "forced boot fault did not emit measured/audited boot fault",
    "PID 1 did not complete the S0 ROM",
    "PID 1 retired too few S0 instructions",
    "ENV_GET did not report expected S0 feature bits",
    "SRAM LD/ST path did not roundtrip the ALU value",
    "OBJECT_CTL did not route through top-level object engine lane",
    "unsupported opcode did not return canonical ENOTSUP",
    "unsupported command did not route through default fail-closed lane",
    "stub resource operation did not fail closed",
    "synthetic event did not wake or mark the parked thread",
    "synthetic stub-engine fault did not emit a structured fault",
    "watchdog-injected stuck command did not reach degraded/fault state",
    "raw physical interrupt/address/DMA/device authority became visible",
    "coherence/TLB/DMA visibility stub paths were not live",
    "not every enabled tile reached reset-stable",
    "tile 1 was not observable, schedulable, and idle",
    "one TID was issued to two tiles",
    "tile 0 did not run PID 1",
    "ENV_GET did not report the two-tile topology",
    "ENV_GET did not report the enabled tile mask",
    "ENV_GET did not report the coherence domain id",
    "ENV_GET did not report the active-window shape",
    "cross-tile wake did not produce exactly one wake",
    "tile-local fault corrupted another tile's scheduler state",
    "4-tile stress configuration did not reach reset-stable",
    "LNP64-RTL-S0 PASS",
]


REQUIRED_MODULES = [
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
]


REQUIRED_B0_SHELL_MODULES = [
    "lnp64_typed_control_validator",
    "lnp64_namespace_dispatch",
    "lnp64_stream_frontend",
    "lnp64_ddr_controller",
    "lnp64_sd_spi_flash",
    "lnp64_boot_image_storage",
]

REQUIRED_S0_OPCODES = [
    "LNP64_OP_NOP",
    "LNP64_OP_LI32",
    "LNP64_OP_ADD",
    "LNP64_OP_JMP",
    "LNP64_OP_LD",
    "LNP64_OP_ST",
    "LNP64_OP_YIELD",
    "LNP64_OP_ENV_GET",
    "LNP64_OP_GET_ERRNO",
    "LNP64_OP_SET_ERRNO",
    "LNP64_OP_OBJECT_CTL",
    "LNP64_OP_FAULT_INJECT",
    "LNP64_OP_UNSUPPORTED",
]


REQUIRED_S0_ROM_OPCODES = [
    "LNP64_OP_NOP",
    "LNP64_OP_LI32",
    "LNP64_OP_ADD",
    "LNP64_OP_JMP",
    "LNP64_OP_LD",
    "LNP64_OP_ST",
    "LNP64_OP_YIELD",
    "LNP64_OP_ENV_GET",
    "LNP64_OP_GET_ERRNO",
    "LNP64_OP_SET_ERRNO",
    "LNP64_OP_OBJECT_CTL",
    "LNP64_OP_UNSUPPORTED",
]

REQUIRED_S0_PROGRAM_ENCODINGS = {
    "LNP64_OP_NOP": r"enc_reg\s*\(\s*8'h00\b",
    "LNP64_OP_LI32": r"enc_ri\s*\(\s*8'h01\b",
    "LNP64_OP_ADD": r"enc_rrr\s*\(\s*8'h10\b",
    "LNP64_OP_JMP": r"enc_branch\s*\(\s*8'h20\b",
    "LNP64_OP_LD": r"enc_mem\s*\(\s*8'h30\b",
    "LNP64_OP_ST": r"enc_mem\s*\(\s*8'h33\b",
    "LNP64_OP_YIELD": r"enc_reg\s*\(\s*8'h06\b",
    "LNP64_OP_ENV_GET": r"enc_rrrr\s*\(\s*8'h56\b",
    "LNP64_OP_GET_ERRNO": r"enc_reg\s*\(\s*8'h38\b",
    "LNP64_OP_SET_ERRNO": r"enc_reg\s*\(\s*8'h39\b",
    "LNP64_OP_OBJECT_CTL": r"enc_rrr\s*\(\s*8'h4b\b",
    "LNP64_OP_UNSUPPORTED": r"enc_reg\s*\(\s*8'hff\b",
}

REQUIRED_S0_DECODE_SUPPORTED_OPCODES = [
    opcode for opcode in REQUIRED_S0_OPCODES if opcode != "LNP64_OP_UNSUPPORTED"
]

REQUIRED_S0_FEATURE_BITS = [
    "LNP64_FEATURE_CORE_TILE",
    "LNP64_FEATURE_DECODE",
    "LNP64_FEATURE_ENV_GET",
    "LNP64_FEATURE_SCHEDULER_STUB",
    "LNP64_FEATURE_EVENT_STUB",
    "LNP64_FEATURE_CAP_STUB",
    "LNP64_FEATURE_DOMAIN_STUB",
    "LNP64_FEATURE_RAS_STUB",
    "LNP64_FEATURE_UART_STUB",
    "LNP64_FEATURE_VMA_ABSENT",
    "LNP64_FEATURE_DMA_ABSENT",
    "LNP64_FEATURE_HEAP_STUB",
    "LNP64_FEATURE_FUTEX_STUB",
    "LNP64_FEATURE_CLASSIFIER_STUB",
    "LNP64_FEATURE_STORAGE_STUB",
    "LNP64_FEATURE_ETH_STUB",
    "LNP64_FEATURE_PCIE_STUB",
]


REQUIRED_TELEMETRY_MODULES = [
    *REQUIRED_B0_SHELL_MODULES,
    "lnp64_cap_engine",
    "lnp64_domain_engine",
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
    "lnp64_storage_stub",
    "lnp64_eth_stub",
    "lnp64_pcie_stub",
]


REQUIRED_CHANNEL_PORTS = {
    "lnp64_core_tile": {
        "tile_enable",
        "release_core",
        "issue_context",
        "topology_tile_count",
        "topology_enabled_tile_mask",
        "topology_coherence_domain_id",
        "topology_active_window_base",
        "topology_active_window_count",
        "cmd_valid",
        "cmd_ready",
        "cmd",
        "rsp_valid",
        "rsp_ready",
        "rsp",
        "tile_reset_stable",
        "tile_idle",
        "tile_running",
        "tile_parked",
        "tile_faulted",
        "retire_submit_valid",
        "retire_submit_record",
        "park_submit_valid",
        "park_submit_record",
        "submit_valid",
        "submit_record",
        "icache_invalidate",
        "icache_invalidate_ack",
        "dcache_writeback",
        "dcache_writeback_ack",
        "tlb_invalidate",
        "tlb_invalidate_ack",
    },
    "lnp64_scheduler": {
        "boot_context",
        "park_submit_valid",
        "park_submit_record",
        "wake_event_valid",
        "wake_event",
        "tile_idle",
        "tile_running",
        "tile_parked",
        "tile_faulted",
        "issue_valid",
        "issue_tid_flat",
        "issue_record",
        "wake_issue_valid",
        "no_duplicate_issue",
        "tile1_schedulable_idle",
        "tile_fault_isolated",
    },
    "lnp64_engine_router": {
        "cmd_valid",
        "cmd_ready",
        "cmd",
        "rsp_valid",
        "rsp_ready",
        "rsp",
        "object_cmd_valid",
        "object_cmd_ready",
        "object_cmd",
        "object_rsp_valid",
        "object_rsp_ready",
        "object_rsp",
        "fault_valid",
        "fault_ready",
        "fault",
    },
    "lnp64_fail_closed_engine": {
        "cmd_valid",
        "cmd_ready",
        "cmd",
        "rsp_valid",
        "rsp_ready",
        "rsp",
        "fault_valid",
        "fault_ready",
        "fault",
    },
    "lnp64_event_router": {"event_valid", "event_ready", "event_record"},
    "lnp64_fault_telemetry": {"fault_valid", "fault_ready", "fault"},
    "lnp64_watchdog": {"fault_valid", "fault_ready", "fault"},
    "lnp64_cap_engine": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_domain_engine": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_object_engine": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_gate_engine": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_process_engine": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_vma_engine": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_service_boundary": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_heap_engine": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
    "lnp64_classifier_servicelet": {"cmd_valid", "cmd_ready", "cmd", "rsp_valid", "rsp_ready", "rsp"},
}


REQUIRED_RECORD_FIELDS = {
    "lnp64_decode_t": {"opcode", "profile", "rd", "rs1", "rs2", "imm", "supported"},
    "lnp64_feature_t": {"isa_version", "profile", "opcode", "feature_bits", "supported"},
    "lnp64_cmd_t": {
        "op_id",
        "tile_id",
        "opcode",
        "profile",
        "pid",
        "tid",
        "domain_id",
        "domain_gen",
        "latency_class",
        "wait_generation",
        "weight_index",
        "virtual_deadline",
        "credential_snapshot_id",
        "result_reg",
        "rights_mask",
        "flags",
        "arg0",
        "arg1",
        "arg2",
        "arg3",
        "arg_block_ptr",
        "arg_block_len",
        "cancel_class",
        "completion_target",
    },
    "lnp64_rsp_t": {
        "op_id",
        "tile_id",
        "pid",
        "tid",
        "domain_id",
        "domain_gen",
        "result_reg",
        "result_value",
        "errno_value",
        "status",
        "event_mask",
    },
    "lnp64_completion_t": {
        "op_id",
        "tile_id",
        "pid",
        "tid",
        "domain_id",
        "domain_gen",
        "target",
        "status",
        "errno_value",
        "value",
    },
    "lnp64_event_t": {
        "event_id",
        "tile_id",
        "op_id",
        "pid",
        "tid",
        "domain_id",
        "domain_gen",
        "event_mask",
        "source",
        "status",
    },
    "lnp64_fault_t": {
        "fault_id",
        "tile_id",
        "op_id",
        "pid",
        "tid",
        "tile_id",
        "domain_id",
        "domain_gen",
        "fault_code",
        "source",
        "detail",
    },
    "lnp64_error_cancel_t": {"op_id", "errno_value", "status", "cancel_class", "revoke_epoch"},
    "lnp64_cap_t": {
        "object_id",
        "object_gen",
        "fdr_gen",
        "domain_id",
        "domain_gen",
        "rights_mask",
        "lineage_epoch",
        "sealed",
        "narrowable",
    },
    "lnp64_object_ref_t": {"object_id", "object_gen", "profile", "length", "bounds_base"},
    "lnp64_control_envelope_t": {
        "version",
        "profile",
        "byte_len",
        "selector",
        "service_generation",
        "payload_ptr",
    },
    "lnp64_namespace_selector_t": {
        "namespace_id",
        "namespace_generation",
        "selector",
        "service_generation",
        "name_hash",
    },
    "lnp64_returned_capability_t": {
        "proposal_id",
        "object_id",
        "object_generation",
        "fdr_generation",
        "domain_id",
        "domain_generation",
        "rights_mask",
    },
    "lnp64_domain_t": {
        "domain_id",
        "domain_gen",
        "parent_domain_id",
        "parent_domain_gen",
        "budget_limit",
        "budget_used",
        "lifecycle_state",
        "assurance_profile",
        "label_id",
    },
    "lnp64_policy_decision_t": {
        "snapshot_id",
        "pid",
        "tid",
        "domain_id",
        "domain_generation",
        "policy_mask",
        "label_id",
    },
    "lnp64_credential_snapshot_t": {
        "snapshot_id",
        "pid",
        "tid",
        "domain_id",
        "domain_generation",
        "credential_generation",
        "delegated_fdr_root",
        "policy_mask",
        "label_id",
    },
    "lnp64_thread_sched_t": {
        "pid",
        "tid",
        "tile_id",
        "domain_id",
        "domain_gen",
        "state",
        "latency_class",
        "wait_generation",
        "weight_index",
        "virtual_deadline",
        "dispatch_eligible",
        "effective_tile_mask",
        "migration_generation",
        "active_location",
    },
    "lnp64_retire_submit_t": {
        "op_id",
        "pid",
        "tid",
        "tile_id",
        "domain_id",
        "domain_gen",
        "pc",
        "opcode",
        "arch_opcode",
        "action",
        "operand_rd",
        "operand_rs1",
        "operand_rs2",
        "operand_rs3",
        "operand_imm",
        "result_valid",
        "result_reg",
        "result_value",
        "errno",
        "status",
        "latency_class",
        "wait_source",
        "event_id",
        "fault_id",
    },
    "lnp64_waitable_t": {
        "wait_id",
        "op_id",
        "pid",
        "tid",
        "domain_id",
        "domain_gen",
        "wait_kind",
        "source_id",
        "timeout_cycles",
    },
    "lnp64_gate_continuation_t": {
        "continuation_id",
        "caller_pid",
        "caller_tid",
        "callee_pid",
        "callee_tid",
        "domain_id",
        "domain_gen",
        "generation",
        "mode",
    },
    "lnp64_process_lifecycle_t": {
        "process_id",
        "process_generation",
        "parent_pid",
        "domain_id",
        "domain_generation",
        "exec_plan_ptr",
        "exec_plan_len",
        "lifecycle_state",
    },
    "lnp64_vma_req_t": {"vma_id", "vma_gen", "domain_id", "domain_gen", "virt_base", "length", "permissions"},
    "lnp64_tlb_cache_invalidate_t": {
        "invalidate_id",
        "tile_id",
        "domain_id",
        "domain_generation",
        "virtual_base",
        "byte_len",
        "scope",
    },
    "lnp64_coherence_txn_t": {
        "txn_id",
        "tile_id",
        "domain_id",
        "domain_generation",
        "address",
        "byte_len",
        "memory_type",
        "ordering",
    },
    "lnp64_heap_alloc_t": {
        "allocation_id",
        "pid",
        "tid",
        "domain_id",
        "domain_generation",
        "size",
        "alignment",
        "heap_profile",
    },
    "lnp64_futex_wait_t": {
        "futex_id",
        "pid",
        "tid",
        "domain_id",
        "domain_generation",
        "address_token",
        "expected_value",
        "timeout_cycles",
    },
    "lnp64_dma_req_t": {
        "dma_id",
        "op_id",
        "domain_id",
        "domain_gen",
        "src_cap",
        "dst_cap",
        "byte_len",
        "latency_class",
    },
    "lnp64_storage_barrier_t": {
        "barrier_id",
        "object_id",
        "object_generation",
        "domain_id",
        "domain_generation",
        "barrier_kind",
    },
    "lnp64_service_txn_t": {
        "service_id",
        "service_generation",
        "op_id",
        "pid",
        "tid",
        "domain_id",
        "domain_generation",
        "payload_ptr",
        "payload_len",
    },
    "lnp64_classifier_action_t": {
        "action_id",
        "table_id",
        "domain_id",
        "domain_generation",
        "action_kind",
        "output_queue",
        "mark",
    },
    "lnp64_watchdog_reset_t": {
        "reset_id",
        "tile_id",
        "op_id",
        "domain_id",
        "domain_generation",
        "reset_kind",
        "degraded_state",
        "reason_code",
    },
    "lnp64_trace_t": {
        "trace_id",
        "tile_id",
        "domain_id",
        "domain_gen",
        "source",
        "severity",
        "counter_value",
        "payload_hash",
    },
    "lnp64_quote_t": {
        "quote_id",
        "build_id",
        "feature_bits",
        "boot_measurement",
        "audit_root",
        "proof_manifest_hash",
    },
    "lnp64_boot_metadata_t": {
        "boot_id",
        "build_id",
        "feature_bits",
        "manifest_hash",
        "image_hash",
        "measurement_root",
    },
}


def fail(message: str) -> None:
    raise SystemExit(f"rtl s0 contract check failed: {message}")


def read_filelist() -> list[Path]:
    if not FILELIST.exists():
        fail(f"missing filelist {FILELIST.relative_to(ROOT)}")
    files = []
    for raw in FILELIST.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        path = ROOT / line
        if not path.exists():
            fail(f"filelist source is missing: {line}")
        files.append(path)
    return files


def find_records(pkg_text: str) -> dict[str, set[str]]:
    records: dict[str, set[str]] = {}
    pattern = re.compile(r"typedef\s+struct\s+packed\s*\{(?P<body>.*?)\}\s+(?P<name>\w+)\s*;", re.S)
    for match in pattern.finditer(pkg_text):
        fields = set(re.findall(r"\blogic(?:\s+\[[^\]]+\])?\s+(\w+)\s*;", match.group("body")))
        records[match.group("name")] = fields
    return records


def find_module_ports(source_text: str, module: str) -> set[str]:
    match = re.search(
        rf"(?m)^\s*module\s+{re.escape(module)}\s*(?:#\s*\(.*?\)\s*)?\((?P<ports>.*?)\)\s*;",
        source_text,
        re.S,
    )
    if not match:
        fail(f"missing module header for {module}")
    ports = set(re.findall(r"\b(?:input|output|inout)\b(?:\s+\w+)*(?:\s+\[[^\]]+\])?\s+(\w+)\b", match.group("ports")))
    ports.update(re.findall(r"\.(\w+)\s*\(", match.group("ports")))
    return ports


def main() -> None:
    sources = read_filelist()
    source_text = "\n".join(path.read_text() for path in sources)
    s0_gate_text = S0_GATE.read_text()
    verilator_common = ROOT / "scripts/rtl_verilator_common.sh"
    verilator_common_text = verilator_common.read_text() if verilator_common.exists() else ""

    defined_modules = set(re.findall(r"(?m)^\s*module\s+(\w+)\b", source_text))
    required_instantiated_modules = REQUIRED_MODULES + REQUIRED_B0_SHELL_MODULES
    missing_modules = sorted(set(required_instantiated_modules) - defined_modules)
    if missing_modules:
        fail(f"missing required S0 module definitions: {', '.join(missing_modules)}")

    for module in required_instantiated_modules:
        if module == "lnp64_top":
            continue
        instance_pattern = re.compile(
            rf"^\s*(?!module\b){re.escape(module)}\s*(?:#\s*\(.*?\)\s*)?\w+\s*\(",
            re.M | re.S,
        )
        if not instance_pattern.search(source_text):
            fail(f"required S0 module is defined but not instantiated: {module}")

    for module in REQUIRED_TELEMETRY_MODULES:
        ports = find_module_ports(source_text, module)
        for required in ("clk", "reset_n", "telemetry_counter", "fault_counter"):
            if required not in ports:
                fail(f"{module} missing required shell status port: {required}")

    for module, required_ports in REQUIRED_CHANNEL_PORTS.items():
        ports = find_module_ports(source_text, module)
        missing_ports = sorted(required_ports - ports)
        if missing_ports:
            fail(f"{module} missing required ready/valid channel ports: {', '.join(missing_ports)}")

    if not PKG.exists():
        fail(f"missing package {PKG.relative_to(ROOT)}")
    records = find_records(PKG.read_text())
    for record, fields in REQUIRED_RECORD_FIELDS.items():
        actual = records.get(record)
        if actual is None:
            fail(f"missing required architectural record: {record}")
        missing_fields = sorted(fields - actual)
        if missing_fields:
            fail(f"{record} missing fields: {', '.join(missing_fields)}")

    for opcode in REQUIRED_S0_OPCODES:
        if not re.search(rf"\b{opcode}\b\s*=", source_text):
            fail(f"missing S0 opcode enum value: {opcode}")

    for opcode in REQUIRED_S0_DECODE_SUPPORTED_OPCODES:
        if not re.search(rf"dec\.opcode\s*==\s*{opcode}\b", source_text):
            fail(f"S0 decode does not mark opcode supported: {opcode}")

    for opcode in REQUIRED_S0_ROM_OPCODES:
        legacy_opcode_literal = rf"\brom\s*=\s*\{{\s*{opcode}\[7:0\]"
        committed_exec_encoding = REQUIRED_S0_PROGRAM_ENCODINGS[opcode]
        if not (
            re.search(legacy_opcode_literal, source_text)
            or re.search(committed_exec_encoding, source_text)
        ):
            fail(f"S0 ROM does not exercise opcode: {opcode}")

    for required in ("lnp64_program_hex", "$readmemh", "program_rom"):
        if required not in source_text:
            fail(f"S0 program image path missing {required}")

    s0_features_match = re.search(
        r"localparam\s+logic\s+\[63:0\]\s+LNP64_S0_FEATURES\s*=\s*(?P<body>.*?);",
        source_text,
        re.DOTALL,
    )
    if not s0_features_match:
        fail("missing LNP64_S0_FEATURES feature mask")
    s0_features_body = s0_features_match.group("body")

    top_feature_mask_match = re.search(
        r"localparam\s+logic\s+\[63:0\]\s+REQUIRED_S0_FEATURE_MASK\s*=\s*(?P<body>.*?);",
        source_text,
        re.DOTALL,
    )
    if not top_feature_mask_match:
        fail("lnp64_top missing REQUIRED_S0_FEATURE_MASK")
    top_feature_mask_body = top_feature_mask_match.group("body")

    for feature in REQUIRED_S0_FEATURE_BITS:
        if not re.search(rf"\b{feature}\b\s*=", source_text):
            fail(f"missing S0 feature bit definition: {feature}")
        if not re.search(rf"\b{feature}\b", s0_features_body):
            fail(f"LNP64_S0_FEATURES does not include required bit: {feature}")
        if not re.search(rf"\b{feature}\b", top_feature_mask_body):
            fail(f"REQUIRED_S0_FEATURE_MASK does not check required bit: {feature}")

    if "(env_features_seen & REQUIRED_S0_FEATURE_MASK) == REQUIRED_S0_FEATURE_MASK" not in source_text:
        fail("lnp64_top does not require the complete S0 feature mask from ENV_GET")

    for marker in (
        "parameter int CORE_TILE_COUNT = 2",
        "parameter int CORE_THREAD_CONTEXT_COUNT = 2",
        "parameter int MAX_SUPPORTED_TILE_COUNT = 4",
        "CORE_TILE_COUNT > MAX_SUPPORTED_TILE_COUNT",
        "CORE_THREAD_CONTEXT_COUNT > 4",
        "for (tile_id = 0; tile_id < CORE_TILE_COUNT",
        ".THREAD_CONTEXT_COUNT(CORE_THREAD_CONTEXT_COUNT)",
        ".TILE_ID(tile_id)",
        "context_active_q",
        "context_parked_q",
        "context_completed_q",
        "context_event_pending_q",
        "context_fault_pending_q",
        "context_record_q",
        "seed_valid",
        "seed_context",
        "activate_context",
        "park_valid",
        "wake_valid",
        "event_valid",
        "fault_valid",
        "activate_valid",
        "complete_valid",
        "collect_valid",
        "advance_valid",
        "SG-SCHED seed context missing scheduler metadata",
        "SG-SCHED seed context tile drift",
        "SG-SCHED context active and parked simultaneously",
        "SG-WAKE completed context retained pending event",
        "SG-SCHED live context missing architectural metadata",
        "retire_submit_next.latency_class = active_thread_context.latency_class",
        "retire_submit_next.wait_source = {32'd0, active_thread_context.wait_generation}",
        "thread_submit_next.wait_generation = active_thread_context.wait_generation",
        "cmd.latency_class = active_thread_context.latency_class",
        "cmd.wait_generation = active_thread_context.wait_generation",
        "cmd.weight_index = active_thread_context.weight_index",
        "cmd.virtual_deadline = active_thread_context.virtual_deadline",
        "thread_submit_next.weight_index = active_thread_context.weight_index",
        "thread_submit_next.virtual_deadline = active_thread_context.virtual_deadline",
        "thread_submit_next.dispatch_eligible = active_thread_context.dispatch_eligible",
        "thread_submit_next.effective_tile_mask = active_thread_context.effective_tile_mask",
        "thread_submit_next.migration_generation = active_thread_context.migration_generation",
        "SG-SCHED live context missing migration generation",
        "context_dispatch_eligible",
        "best_virtual_deadline",
        "deadline_charge_for_weight",
        "selection_virtual_deadline",
        "SG-SCHED barrel skipped earlier virtual deadline",
        "SG-SCHED barrel selected a non-eligible context",
        "SG-SCHED resident context not eligible for this tile",
        "SG-SCHED scheduler PID1 state missing typed metadata",
        "SG-SCHED scheduler park record tile drift",
        "SG-SCHED scheduler issue record missing typed metadata",
        "SG-SCHED scheduler issue record tile drift",
        "SG-SCHED scheduler issued non-eligible record",
        "SG-WAKE scheduler issued wake without valid parked state",
        "SG-SCHED scheduler issued duplicate TID",
        "SG-SCHED engine command lost active thread metadata",
        "core_rsp.tile_id",
        "COHERENCE_DOMAIN_ID",
        "ACTIVE_WINDOW_COUNT",
        "dcache_writeback",
        "tlb_invalidate",
        "icache_invalidate",
        "multicore_no_duplicate_tid",
        "cross_tile_wake_one",
        "tile_fault_isolated",
        "synthetic_event_consumed",
    ):
        if marker not in source_text:
            fail(f"S0 RTL is missing multicore/topology/coherence marker: {marker}")

    if "sim_fault_inject" not in source_text or "synthetic stub-engine fault" not in source_text:
        fail("S0 synthetic fault-injection hook is not covered by the testbench")

    for marker in REQUIRED_ACCEPTANCE_MARKERS:
        if marker not in source_text:
            fail(f"S0 acceptance testbench is missing marker: {marker}")

    if (
        verilator_common.exists()
        and "verilator --lint-only" not in s0_gate_text
        and "verilator --lint-only" not in verilator_common_text
    ):
        fail("S0 gate script is missing marker: verilator --lint-only")

    if "verilator --binary" not in s0_gate_text and "verilator --binary" not in verilator_common_text:
        fail("S0 gate script is missing marker: verilator --binary")

    for marker in (
        "--top-module lnp64_s0_tb",
        "tests/rtl/s0_filelist.f",
        "grep -q \"LNP64-RTL-S0 PASS\"",
        "rtl s0 gate ok",
    ):
        if marker not in s0_gate_text:
            fail(f"S0 gate script is missing marker: {marker}")

    print("rtl s0 contract ok")


if __name__ == "__main__":
    main()

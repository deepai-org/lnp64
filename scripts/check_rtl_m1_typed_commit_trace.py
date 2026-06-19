#!/usr/bin/env python3
"""Check the M1 RTL typed capability commit trace.

This is intentionally narrow plumbing for the first SG-AUTH transition slice.
It does not expand the global proof manifest. It checks that the M1 RTL emits a
typed commit path corresponding to the Lean M1 transition model:
capDup, capDupDenied, capSend, capRecv, push, pull, rejectFull, capRevoke,
rejectStale.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "rtl/schema/lnp64_shared_schema.json"
LEAN_M1_MODEL = ROOT / "formal/M1TransitionInvariantModel.lean"
RTL_M1_ENGINE = ROOT / "rtl/engines/lnp64_m1_pingpong.sv"
RTL_M1_TB = ROOT / "rtl/sim/lnp64_m1_tb.sv"
RTL_M1_ASSERTIONS = ROOT / "formal/rtl_assertions/lnp64_m1_assertions.sv"
RTL_PKG = ROOT / "rtl/include/lnp64_pkg.sv"
DEFAULT_M1_TRACE_LOG = Path("/tmp/lnp64_rtl_m1_typed_commit.log")
DEFAULT_TYPED_COMMIT_SEEDS = (
    "0 1 7 42 255 1024 4095 4096 65536 1048576 16777216 134217728 268435456 536870912"
)

RIGHT_PUSH = 0x1
RIGHT_PULL = 0x2
RIGHT_DUP = 0x4
RIGHT_MINT = 0x8
ROOT_RIGHTS = RIGHT_PUSH | RIGHT_PULL | RIGHT_DUP | RIGHT_MINT

ERR_OK = 0
ERR_EPERM = 1
ERR_EAGAIN = 11
ERR_EREVOKED = 122

CAP_FIELDS = (
    "object_id",
    "object_gen",
    "fdr_gen",
    "domain_id",
    "domain_gen",
    "rights_mask",
    "lineage_epoch",
    "sealed",
    "status",
)

COMMIT_FIELDS = ("op",) + CAP_FIELDS

STATE_PROJECTION_FIELDS = (
    "op",
    "status",
    "object_gen",
    "created_object_created",
    "created_object_gen",
    "root_object_id",
    "root_generation",
    "root_domain_id",
    "root_lineage_epoch",
    "root_sealed",
    "root_rights",
    "consumer_object_id",
    "consumer_generation",
    "consumer_domain_id",
    "consumer_lineage_epoch",
    "consumer_sealed",
    "consumer_rights",
    "sent_valid",
    "sent_object_id",
    "sent_generation",
    "sent_domain_id",
    "sent_lineage_epoch",
    "sent_sealed",
    "sent_rights",
    "minted_valid",
    "minted_object_id",
    "minted_generation",
    "minted_domain_id",
    "minted_lineage_epoch",
    "minted_sealed",
    "minted_rights",
    "wake_pending",
    "transfer_valid",
    "stale_rejected",
    "revoked_rejected",
    "failed_no_authority",
    "full_was_explicit",
    "has_revoked_generation",
    "revoked_generation",
)

AUTHORITY_STATE_PROJECTION_FIELDS = (
    "root_object_id",
    "root_generation",
    "root_domain_id",
    "root_lineage_epoch",
    "root_sealed",
    "root_rights",
    "consumer_object_id",
    "consumer_generation",
    "consumer_domain_id",
    "consumer_lineage_epoch",
    "consumer_sealed",
    "consumer_rights",
    "sent_valid",
    "sent_object_id",
    "sent_generation",
    "sent_domain_id",
    "sent_lineage_epoch",
    "sent_sealed",
    "sent_rights",
    "minted_valid",
    "minted_object_id",
    "minted_generation",
    "minted_domain_id",
    "minted_lineage_epoch",
    "minted_sealed",
    "minted_rights",
)

M1_OP_KEYS = (
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
)

M1_STATUS_KEYS = ("ok", "eperm", "eagain", "erevoked")

M1_LEAN_COMMIT_RECORD_FIELDS = (
    "op",
    "objectId",
    "objectGeneration",
    "fdrGeneration",
    "domainId",
    "domainGeneration",
    "rights",
    "lineageEpoch",
    "sealed",
    "status",
)

M1_LEAN_RTL_STATE_PROJECTION_FIELDS = (
    "objectGeneration",
    "createdObjectCreated",
    "createdObjectGeneration",
    "rootCap",
    "consumerCap",
    "sentCap",
    "mintedCap",
    "wakePending",
    "transferValid",
    "staleRejected",
    "revokedRejected",
    "failedNoAuthority",
    "fullWasExplicit",
    "hasRevokedGeneration",
    "revokedGeneration",
)

M1_TYPED_COMMIT_LEAN_THEOREMS = (
    "typed_commit_transition_refines_step",
    "typed_commit_transition_preserves_invariant",
    "typed_commit_transition_status_matches_op",
    "rtl_m1_refinement_step_refines_lean_step",
    "rtl_m1_refinement_step_refines_commit_projection_op",
    "rtl_m1_refinement_step_status_matches_op",
    "rtl_m1_refinement_step_projection_faithful",
    "rtl_m1_refinement_step_preserves_sg_auth_invariant",
    "rtl_m1_commit_projection_from_packed_bits_within_schema_width",
    "rtl_m1_state_projection_from_packed_bits_within_schema_width",
    "rtl_m1_commit_projection_from_packed_bits_rights_modeled",
    "rtl_m1_state_projection_from_packed_bits_rights_modeled",
    "rtl_m1_packed_refinement_step_refines_lean_step",
    "rtl_m1_packed_refinement_step_status_matches_op",
    "rtl_m1_packed_refinement_step_preserves_sg_auth_invariant",
    "m1_t3_typed_commit_transition_refines_step_for_reachable",
    "m1_t3_typed_commit_transition_preserves_invariant_for_reachable",
    "m1_t3_typed_commit_transition_status_matches_op_for_reachable",
    "m1_t3_rtl_m1_refinement_step_preserves_sg_auth_invariant_for_reachable",
    "m1_t3_rtl_m1_refinement_step_refines_commit_projection_op_for_reachable",
    "m1_t3_rtl_m1_refinement_step_refines_preserves_and_satisfies_postcondition_for_reachable",
    "m1_t3_rtl_m1_refinement_step_status_matches_op_for_reachable",
    "m1_t3_cap_send_requires_current_authority_for_reachable",
    "m1_t3_cap_recv_requires_current_authority_for_reachable",
    "m1_t3_revoke_invalidates_outstanding_main_object_transfer_for_reachable",
    "step_cap_dup_denied_preserves_authority_slots",
    "step_cap_recv_empty_preserves_authority_slots",
    "step_object_create_denied_preserves_authority_slots",
    "step_reject_stale_preserves_authority_slots",
    "step_reject_revoked_preserves_authority_slots",
    "step_reject_full_preserves_authority_slots",
    "m1_t3_failed_authority_operations_preserve_authority_slots_for_reachable",
    "m1_t3_typed_commit_failed_authority_transition_preserves_authority_slots_for_reachable",
    "m1_t3_typed_commit_non_ok_status_preserves_authority_slots_for_reachable",
    "rtl_m1_refinement_failed_authority_transition_preserves_authority_projection",
    "rtl_m1_refinement_non_ok_status_preserves_authority_projection",
    "rtl_m1_refinement_step_satisfies_postcondition",
    "rtl_m1_refinement_cap_dup_post_consumer_matches_commit_projection",
    "rtl_m1_refinement_cap_send_post_sent_matches_commit_projection",
    "rtl_m1_refinement_cap_recv_post_consumer_matches_commit_projection",
    "rtl_m1_refinement_object_create_post_minted_matches_commit_projection",
    "rtl_m1_refinement_push_post_wake_matches_commit_projection",
    "rtl_m1_refinement_pull_post_wake_matches_commit_projection",
    "rtl_m1_refinement_reject_full_post_failure_matches_commit_projection",
    "rtl_m1_refinement_reject_stale_post_failure_matches_commit_projection",
    "rtl_m1_refinement_cap_dup_denied_post_failure_matches_commit_projection",
    "rtl_m1_refinement_cap_revoke_post_generation_matches_commit_projection",
    "rtlM1CommitPackedSchema_width",
    "rtlM1StateProjectionPackedSchema_width",
    "rtlM1CommitSchemaToLeanProjection_covers_schema",
    "rtlM1CommitSchemaToLeanProjection_targets_commit_projection",
    "rtlM1StateProjectionSchemaToLeanProjection_covers_schema",
    "rtlM1StateProjectionSchemaToLeanProjection_targets_state_projection",
    "rtlM1CommitPackedLayout_within_schema_width",
    "rtlM1StateProjectionPackedLayout_within_schema_width",
    "rtlM1CommitPackedLayout_covers_schema_width",
    "rtlM1StateProjectionPackedLayout_covers_schema_width",
)

M1_SCHEMA_TO_LEAN_COMMIT_FIELDS = {
    "op": "op",
    "object_id": "objectId",
    "object_gen": "objectGeneration",
    "fdr_gen": "fdrGeneration",
    "domain_id": "domainId",
    "domain_gen": "domainGeneration",
    "rights_mask": "rights",
    "lineage_epoch": "lineageEpoch",
    "sealed": "sealed",
    "status": "status",
}

M1_STATE_CAP_PROJECTION_FIELDS = (
    "object_id",
    "generation",
    "domain_id",
    "lineage_epoch",
    "sealed",
    "rights",
)

M1_SCHEMA_TO_LEAN_STATE_PROJECTION_PATHS = (
    ("op", "transitionTag.op"),
    ("status", "transitionTag.status"),
    ("object_gen", "objectGeneration"),
    ("created_object_created", "createdObjectCreated"),
    ("created_object_gen", "createdObjectGeneration"),
    ("root_object_id", "rootCap.objectId"),
    ("root_generation", "rootCap.generation"),
    ("root_domain_id", "rootCap.ownerDomain"),
    ("root_lineage_epoch", "rootCap.lineageEpoch"),
    ("root_sealed", "rootCap.sealed"),
    ("root_rights", "rootCap.rights"),
    ("consumer_object_id", "consumerCap.objectId"),
    ("consumer_generation", "consumerCap.generation"),
    ("consumer_domain_id", "consumerCap.ownerDomain"),
    ("consumer_lineage_epoch", "consumerCap.lineageEpoch"),
    ("consumer_sealed", "consumerCap.sealed"),
    ("consumer_rights", "consumerCap.rights"),
    ("sent_valid", "sentCap.valid"),
    ("sent_object_id", "sentCap.objectId"),
    ("sent_generation", "sentCap.generation"),
    ("sent_domain_id", "sentCap.ownerDomain"),
    ("sent_lineage_epoch", "sentCap.lineageEpoch"),
    ("sent_sealed", "sentCap.sealed"),
    ("sent_rights", "sentCap.rights"),
    ("minted_valid", "mintedCap.valid"),
    ("minted_object_id", "mintedCap.objectId"),
    ("minted_generation", "mintedCap.generation"),
    ("minted_domain_id", "mintedCap.ownerDomain"),
    ("minted_lineage_epoch", "mintedCap.lineageEpoch"),
    ("minted_sealed", "mintedCap.sealed"),
    ("minted_rights", "mintedCap.rights"),
    ("wake_pending", "wakePending"),
    ("transfer_valid", "transferValid"),
    ("stale_rejected", "staleRejected"),
    ("revoked_rejected", "revokedRejected"),
    ("failed_no_authority", "failedNoAuthority"),
    ("full_was_explicit", "fullWasExplicit"),
    ("has_revoked_generation", "hasRevokedGeneration"),
    ("revoked_generation", "revokedGeneration"),
)


def lean_state_projection_fields_from_schema(schema_fields: tuple[str, ...]) -> tuple[str, ...]:
    """Map the flat RTL state projection schema onto the Lean projection fields."""
    remaining = list(schema_fields)
    lean_fields: list[str] = []

    def take_exact(expected: tuple[str, ...], lean_field: str) -> None:
        actual = tuple(remaining[:len(expected)])
        if actual != expected:
            fail(
                "M1 state projection schema no longer maps cleanly to Lean "
                f"{lean_field}: {actual!r} != {expected!r}"
            )
        del remaining[:len(expected)]
        lean_fields.append(lean_field)

    take_exact(("op", "status"), "transitionTag")
    take_exact(("object_gen",), "objectGeneration")
    take_exact(("created_object_created",), "createdObjectCreated")
    take_exact(("created_object_gen",), "createdObjectGeneration")
    take_exact(tuple(f"root_{field}" for field in M1_STATE_CAP_PROJECTION_FIELDS), "rootCap")
    take_exact(tuple(f"consumer_{field}" for field in M1_STATE_CAP_PROJECTION_FIELDS), "consumerCap")
    take_exact(
        ("sent_valid",) + tuple(f"sent_{field}" for field in M1_STATE_CAP_PROJECTION_FIELDS),
        "sentCap",
    )
    take_exact(
        ("minted_valid",) + tuple(f"minted_{field}" for field in M1_STATE_CAP_PROJECTION_FIELDS),
        "mintedCap",
    )
    take_exact(("wake_pending",), "wakePending")
    take_exact(("transfer_valid",), "transferValid")
    take_exact(("stale_rejected",), "staleRejected")
    take_exact(("revoked_rejected",), "revokedRejected")
    take_exact(("failed_no_authority",), "failedNoAuthority")
    take_exact(("full_was_explicit",), "fullWasExplicit")
    take_exact(("has_revoked_generation",), "hasRevokedGeneration")
    take_exact(("revoked_generation",), "revokedGeneration")
    if remaining:
        fail(f"M1 state projection schema has unmapped fields: {tuple(remaining)!r}")
    return tuple(field for field in lean_fields if field != "transitionTag")


@dataclass(frozen=True)
class CommitOps:
    cap_dup: int
    cap_send: int
    cap_recv: int
    cap_revoke: int
    reject_stale: int
    push: int
    pull: int
    reject_full: int
    cap_dup_denied: int
    object_create: int

    @property
    def expected_sequence(self) -> list[int]:
        return [
            self.cap_dup,
            self.cap_send,
            self.cap_recv,
            self.push,
            self.pull,
            self.reject_full,
            self.object_create,
            self.cap_revoke,
            self.reject_stale,
        ]

    @property
    def denied_sequence(self) -> list[int]:
        return [self.cap_dup_denied]

    @property
    def valid_ops(self) -> set[int]:
        return set(self.expected_sequence + self.denied_sequence)


@dataclass(frozen=True)
class M1OpMapping:
    key: str
    sv: str
    lean_op: str
    lean_commit_op: str
    lean_transition: str


@dataclass(frozen=True)
class M1StatusMapping:
    key: str
    sv_errno: str
    lean_status: str


@dataclass(frozen=True)
class Cap:
    object_id: int
    object_gen: int
    fdr_gen: int
    domain_id: int
    domain_gen: int
    rights_mask: int
    lineage_epoch: int
    sealed: int


@dataclass
class M1State:
    object_gen: int
    root_cap: Cap
    consumer_cap: Cap | None = None
    sent_cap: Cap | None = None
    minted_cap: Cap | None = None
    created_object_created: bool = False
    created_object_gen: int = 1
    queue_full: bool = False
    wake_pending: bool = False
    transfer_valid: bool = False
    stale_rejected: bool = False
    revoked_rejected: bool = False
    failed_no_authority: bool = False
    full_was_explicit: bool = False
    revoked_gen: int | None = None


def fail(message: str) -> None:
    print(f"rtl m1 typed commit trace check failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def parse_sv_int(value: str) -> int:
    value = value.replace("_", "")
    if "'" not in value:
        return int(value, 0)
    _width, literal = value.split("'", 1)
    base = literal[0].lower()
    digits = literal[1:]
    if base == "d":
        return int(digits, 10)
    if base == "h":
        return int(digits, 16)
    if base == "b":
        return int(digits, 2)
    fail(f"unsupported SystemVerilog integer literal {value!r}")


def parse_schema_field(entry: str) -> tuple[str, int]:
    name, raw_width = entry.split(":", 1)
    try:
        width = int(raw_width, 10)
    except ValueError as exc:
        fail(f"invalid schema field width in {entry!r}: {exc}")
    if width <= 0:
        fail(f"invalid nonpositive schema field width in {entry!r}")
    return name, width


def load_lean_model_source() -> str:
    try:
        return LEAN_M1_MODEL.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"could not read Lean M1 transition model: {exc}")


def require_trace_display_payload_source(
    tb_source: str,
    marker: str,
    source_signal: str,
    message: str,
) -> None:
    marker_index = tb_source.find(marker)
    if marker_index < 0:
        fail(message)
    display_end = tb_source.find(");", marker_index)
    if display_end < 0:
        fail(message)
    display_call = tb_source[marker_index:display_end]
    if re.search(rf"\b{re.escape(source_signal)}\b", display_call) is None:
        fail(message)


def check_rtl_state_projection_boundary_sources(
    engine_source: str,
    tb_source: str,
    assertion_source: str,
    expected_commit_fields: tuple[str, ...],
    expected_state_fields: tuple[str, ...],
) -> None:
    if "output lnp64_m1_state_projection_t typed_state_projection" not in engine_source:
        fail("M1 engine no longer exposes schema-owned typed_state_projection")
    if "lnp64_m1_cap_commit_t typed_commit" not in tb_source:
        fail("M1 testbench no longer declares the schema-owned typed_commit record")
    required_pre_state_sampling = (
        "lnp64_m1_state_projection_t sampled_pre_state_projection",
        "lnp64_m1_state_projection_t typed_pre_state_projection",
        "typed_pre_state_projection = sampled_pre_state_projection",
        "typed_pre_state_projection.op = typed_commit.op",
        "typed_pre_state_projection.status = typed_commit.status",
        "sampled_pre_state_projection <= typed_state_projection",
    )
    missing_pre_state_sampling = [
        source
        for source in required_pre_state_sampling
        if source not in tb_source
    ]
    if missing_pre_state_sampling:
        fail(
            "M1 testbench no longer derives emitted pre-state projection by sampling "
            f"typed_state_projection before the commit: {missing_pre_state_sampling}"
        )
    if ".typed_state_projection(typed_state_projection)" not in tb_source:
        fail("M1 testbench no longer connects the typed_state_projection boundary")
    if "input lnp64_m1_state_projection_t typed_state_projection" not in assertion_source:
        fail("M1 assertions no longer consume the schema-owned typed_state_projection")
    if ".typed_state_projection(typed_state_projection)" not in tb_source:
        fail("M1 testbench no longer passes typed_state_projection into assertions")
    required_faithfulness_ports = (
        "input logic [31:0] queue_generation",
        "input logic [31:0] producer_fd_generation",
        "input logic [31:0] consumer_fd_generation",
        "input logic [63:0] producer_rights",
        "input logic [63:0] consumer_rights",
        "input logic sent_cap_valid",
        "input logic minted_cap_valid",
        "input lnp64_cap_t sent_cap_state",
        "input lnp64_cap_t minted_cap_state",
        "input logic created_object_created",
        "input logic [31:0] created_object_generation",
    )
    missing_faithfulness_ports = [
        port
        for port in required_faithfulness_ports
        if port not in assertion_source
    ]
    if missing_faithfulness_ports:
        fail(
            "M1 assertions no longer receive real RTL authority state for projection faithfulness: "
            f"{missing_faithfulness_ports}"
        )
    required_faithfulness_connections = (
        ".queue_generation(dut.queue_generation)",
        ".producer_fd_generation(dut.producer_fd_generation)",
        ".consumer_fd_generation(dut.consumer_fd_generation)",
        ".producer_rights(dut.producer_rights)",
        ".consumer_rights(dut.consumer_rights)",
        ".sent_cap_valid(dut.sent_cap_valid)",
        ".minted_cap_valid(dut.minted_cap_valid)",
        ".sent_cap_state(dut.sent_cap_state)",
        ".minted_cap_state(dut.minted_cap_state)",
        ".created_object_created(dut.created_object_created)",
        ".created_object_generation(dut.created_object_generation)",
    )
    missing_faithfulness_connections = [
        connection
        for connection in required_faithfulness_connections
        if connection not in tb_source
    ]
    if missing_faithfulness_connections:
        fail(
            "M1 testbench no longer wires real RTL authority state into projection faithfulness assertions: "
            f"{missing_faithfulness_connections}"
        )
    if (
        "TTRACE_M1_BITS" not in tb_source
        or "TTRACE_M1_PRE_STATE_BITS" not in tb_source
        or "TTRACE_M1_STATE_BITS" not in tb_source
    ):
        fail("M1 testbench no longer emits packed bit records for commit and pre/post state projections")
    required_bit_width_sources = (
        "$bits(lnp64_m1_cap_commit_t)",
        "$bits(lnp64_m1_state_projection_t)",
    )
    missing_bit_width_sources = [
        source
        for source in required_bit_width_sources
        if source not in tb_source
    ]
    if missing_bit_width_sources:
        fail(
            "M1 testbench no longer emits schema-owned packed bit widths: "
            f"{missing_bit_width_sources}"
        )
    require_trace_display_payload_source(
        tb_source,
        "TTRACE_M1_BITS",
        "typed_commit",
        "M1 testbench no longer emits packed commit bits from typed_commit",
    )
    require_trace_display_payload_source(
        tb_source,
        "TTRACE_M1_PRE_STATE_BITS",
        "typed_pre_state_projection",
        "M1 testbench no longer emits packed pre-state bits from typed_pre_state_projection",
    )
    require_trace_display_payload_source(
        tb_source,
        "TTRACE_M1_STATE_BITS",
        "typed_state_projection",
        "M1 testbench no longer emits packed state bits from typed_state_projection",
    )
    require_trace_display_payload_source(
        tb_source,
        "TTRACE_M1_PRE_STATE",
        "typed_pre_state_projection",
        "M1 testbench no longer emits every pre-state trace field from typed_pre_state_projection",
    )
    if "m1_authority_projection_slots_match" not in assertion_source:
        fail("M1 assertions no longer check non-OK authority projection preservation")
    required_engine_projection_sources = (
        "typed_state_projection.sent_object_id = sent_cap_state.object_id",
        "typed_state_projection.sent_generation = sent_cap_state.fdr_gen",
        "typed_state_projection.sent_domain_id = sent_cap_state.domain_id",
        "typed_state_projection.sent_lineage_epoch = sent_cap_state.lineage_epoch",
        "typed_state_projection.sent_sealed = sent_cap_state.sealed",
        "typed_state_projection.sent_rights = sent_cap_state.rights_mask",
        "typed_state_projection.minted_object_id = minted_cap_state.object_id",
        "typed_state_projection.minted_generation = minted_cap_state.fdr_gen",
        "typed_state_projection.minted_domain_id = minted_cap_state.domain_id",
        "typed_state_projection.minted_lineage_epoch = minted_cap_state.lineage_epoch",
        "typed_state_projection.minted_sealed = minted_cap_state.sealed",
        "typed_state_projection.minted_rights = minted_cap_state.rights_mask",
    )
    missing_engine_projection_sources = [
        source
        for source in required_engine_projection_sources
        if source not in engine_source
    ]
    if missing_engine_projection_sources:
        fail(
            "M1 typed_state_projection no longer derives sent/minted authority fields "
            f"from explicit RTL cap-state slots: {missing_engine_projection_sources}"
        )
    missing_commit_fields = [
        field
        for field in expected_commit_fields
        if f"typed_commit.{field}" not in tb_source
    ]
    if missing_commit_fields:
        fail(
            "M1 testbench no longer emits every commit trace field from typed_commit: "
            f"{missing_commit_fields}"
        )
    missing_trace_fields = [
        field
        for field in expected_state_fields
        if f"typed_state_projection.{field}" not in tb_source
    ]
    if missing_trace_fields:
        fail(
            "M1 testbench no longer emits every state trace field from typed_state_projection: "
            f"{missing_trace_fields}"
        )
    missing_pre_trace_fields = [
        field
        for field in expected_state_fields
        if f"typed_pre_state_projection.{field}" not in tb_source
    ]
    if missing_pre_trace_fields:
        fail(
            "M1 testbench no longer emits every pre-state trace field from typed_pre_state_projection: "
            f"{missing_pre_trace_fields}"
        )
    required_assertion_fields = (
        "op",
        "status",
        "object_gen",
        "root_object_id",
        "root_generation",
        "root_domain_id",
        "root_lineage_epoch",
        "root_sealed",
        "root_rights",
        "consumer_object_id",
        "consumer_generation",
        "consumer_domain_id",
        "consumer_lineage_epoch",
        "consumer_sealed",
        "consumer_rights",
        "sent_valid",
        "sent_object_id",
        "sent_generation",
        "sent_domain_id",
        "sent_lineage_epoch",
        "sent_sealed",
        "sent_rights",
        "minted_valid",
        "minted_object_id",
        "minted_generation",
        "minted_domain_id",
        "minted_lineage_epoch",
        "minted_sealed",
        "minted_rights",
    )
    missing_assertion_fields = [
        field
        for field in required_assertion_fields
        if f"typed_state_projection.{field}" not in assertion_source
    ]
    if missing_assertion_fields:
        fail(
            "M1 assertions no longer mediate authority fields through typed_state_projection: "
            f"{missing_assertion_fields}"
        )
    required_assertion_hooks = (
        "M1 typed state projection object generation did not match RTL queue_generation",
        "M1 typed state projection root generation did not match RTL producer_fd_generation",
        "M1 typed state projection consumer generation did not match RTL consumer_fd_generation",
        "M1 typed state projection root rights did not match RTL producer_rights",
        "M1 typed state projection consumer rights did not match RTL consumer_rights",
        "M1 typed state projection sent_valid did not match RTL sent_cap_valid",
        "M1 typed state projection minted_valid did not match RTL minted_cap_valid",
        "M1 typed commit status did not match operation",
        "M1 invalid sent-cap state retained authority bits",
        "M1 sent-cap projection did not match RTL sent_cap_state",
        "M1 invalid minted-cap state retained authority bits",
        "M1 minted-cap projection did not match RTL minted_cap_state",
        "M1 sent-cap validity set outside capSend owner path",
        "M1 sent-cap validity cleared outside capRecv owner path",
        "M1 transfer-valid witness set outside capSend owner path",
        "M1 minted-cap validity set outside objectCreate owner path",
        "M1 created-object witness set outside objectCreate owner path",
    )
    missing_assertion_hooks = [
        hook
        for hook in required_assertion_hooks
        if hook not in assertion_source
    ]
    if missing_assertion_hooks:
        fail(
            "M1 assertions no longer mediate transfer/mint validity transitions: "
            f"{missing_assertion_hooks}"
        )


def parse_sv_packed_struct_fields(source: str, typedef_name: str) -> tuple[tuple[str, int], ...]:
    match = re.search(
        rf"(?ms)typedef\s+struct\s+packed\s*\{{(?P<body>[^}}]*)\}}\s*{re.escape(typedef_name)}\s*;",
        source,
    )
    if not match:
        fail(f"RTL package is missing packed typedef {typedef_name}")
    fields: list[tuple[str, int]] = []
    for raw_line in match.group("body").splitlines():
        line = raw_line.split("//", 1)[0].strip()
        if not line:
            continue
        field_match = re.fullmatch(
            r"logic(?:\s*\[\s*(?P<msb>[0-9]+)\s*:\s*(?P<lsb>[0-9]+)\s*\])?\s+"
            r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;",
            line,
        )
        if not field_match:
            fail(f"could not parse field in RTL packed typedef {typedef_name}: {raw_line!r}")
        msb = field_match.group("msb")
        lsb = field_match.group("lsb")
        width = 1 if msb is None else abs(int(msb) - int(lsb)) + 1
        fields.append((field_match.group("name"), width))
    if not fields:
        fail(f"RTL packed typedef {typedef_name} has no parsed fields")
    return tuple(fields)


def check_rtl_packed_typedefs_match_schema_sources(
    pkg_source: str,
    commit_field_specs: tuple[tuple[str, int], ...],
    state_field_specs: tuple[tuple[str, int], ...],
) -> None:
    commit_typedef = parse_sv_packed_struct_fields(pkg_source, "lnp64_m1_cap_commit_t")
    if commit_typedef != commit_field_specs:
        fail(
            "RTL lnp64_m1_cap_commit_t packed typedef drifted from shared schema: "
            f"{commit_typedef!r} != {commit_field_specs!r}"
        )
    state_typedef = parse_sv_packed_struct_fields(pkg_source, "lnp64_m1_state_projection_t")
    if state_typedef != state_field_specs:
        fail(
            "RTL lnp64_m1_state_projection_t packed typedef drifted from shared schema: "
            f"{state_typedef!r} != {state_field_specs!r}"
        )


def check_rtl_packed_typedefs_match_schema(
    commit_field_specs: tuple[tuple[str, int], ...],
    state_field_specs: tuple[tuple[str, int], ...],
) -> None:
    try:
        pkg_source = RTL_PKG.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"could not read RTL package: {exc}")
    check_rtl_packed_typedefs_match_schema_sources(
        pkg_source,
        commit_field_specs,
        state_field_specs,
    )


def check_rtl_state_projection_boundary(
    expected_commit_fields: tuple[str, ...],
    expected_state_fields: tuple[str, ...],
) -> None:
    try:
        engine_source = RTL_M1_ENGINE.read_text(encoding="utf-8")
        tb_source = RTL_M1_TB.read_text(encoding="utf-8")
        assertion_source = RTL_M1_ASSERTIONS.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"could not read M1 RTL sources: {exc}")
    check_rtl_state_projection_boundary_sources(
        engine_source,
        tb_source,
        assertion_source,
        expected_commit_fields,
        expected_state_fields,
    )


def load_lean_inductive_constructors(source: str, name: str) -> set[str]:
    match = re.search(
        rf"(?ms)^inductive\s+{re.escape(name)}\b(?P<body>.*?)(?=^(?:deriving\b|inductive\b|structure\b|def\b|abbrev\b|theorem\b|lemma\b))",
        source,
    )
    if not match:
        fail(f"Lean M1 transition model is missing inductive {name}")
    return set(re.findall(r"(?m)^\s*\|\s*([A-Za-z0-9_']+)\b", match.group("body")))


def load_lean_structure_fields(source: str, name: str) -> tuple[str, ...]:
    match = re.search(rf"(?ms)^structure\s+{re.escape(name)}\s+where\b(?P<body>.*?)(?=^deriving\b)", source)
    if not match:
        fail(f"Lean M1 transition model is missing structure {name}")
    return tuple(
        field
        for field in re.findall(r"(?m)^\s*([A-Za-z0-9_']+)\s*:", match.group("body"))
    )


def load_lean_packed_schema(source: str, name: str) -> tuple[tuple[str, int], ...]:
    match = re.search(
        rf"(?ms)^def\s+{re.escape(name)}\s*:\s*List\s*\(String\s*×\s*Nat\)\s*:=\s*\[(?P<body>.*?)^\]",
        source,
    )
    if not match:
        fail(f"Lean M1 transition model is missing packed schema {name}")
    fields = tuple(
        (field_name, int(width))
        for field_name, width in re.findall(r'\("([^"]+)",\s*([0-9]+)\)', match.group("body"))
    )
    if not fields:
        fail(f"Lean packed schema {name} has no parsed fields")
    return fields


def load_lean_string_pair_list(source: str, name: str) -> tuple[tuple[str, str], ...]:
    match = re.search(
        rf"(?ms)^def\s+{re.escape(name)}\s*:\s*List\s*\(String\s*×\s*String\)\s*:=\s*\[(?P<body>.*?)^\]",
        source,
    )
    if not match:
        fail(f"Lean M1 transition model is missing string pair list {name}")
    pairs = tuple(
        (left, right)
        for left, right in re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', match.group("body"))
    )
    if not pairs:
        fail(f"Lean string pair list {name} has no parsed entries")
    return pairs


def load_lean_packed_schema_width_theorem(source: str, theorem_name: str, schema_name: str) -> int:
    match = re.search(
        rf"(?ms)^theorem\s+{re.escape(theorem_name)}\s*:\s*"
        rf"packedSchemaWidth\s+{re.escape(schema_name)}\s*=\s*(?P<width>[0-9]+)\s*:=\s*by\b",
        source,
    )
    if not match:
        fail(
            "Lean M1 transition model is missing packed schema width theorem "
            f"{theorem_name} for {schema_name}"
        )
    return int(match.group("width"))


def load_lean_theorems(source: str) -> set[str]:
    return set(re.findall(r"(?m)^(?:theorem|lemma)\s+([A-Za-z0-9_']+)\b", source))


def require_string_mapping_field(entry: object, field: str, label: str) -> str:
    if not isinstance(entry, dict):
        fail(f"shared schema {label} entry is not an object: {entry!r}")
    value = entry.get(field)
    if not isinstance(value, str) or not value:
        fail(f"shared schema {label} entry has invalid {field}: {entry!r}")
    return value


def load_m1_op_mappings(contract: dict[str, object]) -> tuple[M1OpMapping, ...]:
    raw_mappings = contract.get("op_mappings")
    if not isinstance(raw_mappings, list) or not raw_mappings:
        fail("shared schema M1 contract is missing op_mappings")
    mappings = tuple(
        M1OpMapping(
            key=require_string_mapping_field(entry, "key", "M1 op_mappings"),
            sv=require_string_mapping_field(entry, "sv", "M1 op_mappings"),
            lean_op=require_string_mapping_field(entry, "lean_op", "M1 op_mappings"),
            lean_commit_op=require_string_mapping_field(entry, "lean_commit_op", "M1 op_mappings"),
            lean_transition=require_string_mapping_field(entry, "lean_transition", "M1 op_mappings"),
        )
        for entry in raw_mappings
    )
    keys = tuple(mapping.key for mapping in mappings)
    if keys != M1_OP_KEYS:
        fail(f"shared schema M1 op mapping keys drifted: {keys!r} != {M1_OP_KEYS!r}")
    sv_names = [mapping.sv for mapping in mappings]
    if len(sv_names) != len(set(sv_names)):
        fail(f"shared schema M1 op mappings contain duplicate SV names: {sv_names!r}")
    for mapping in mappings:
        expected_lean_name = lean_constructor_name_from_key(mapping.key)
        actual_lean_names = (mapping.lean_op, mapping.lean_commit_op, mapping.lean_transition)
        if actual_lean_names != (expected_lean_name, expected_lean_name, expected_lean_name):
            fail(
                "shared schema M1 op mapping no longer names the exact Lean constructor "
                f"for key {mapping.key!r}: {actual_lean_names!r} != "
                f"{(expected_lean_name, expected_lean_name, expected_lean_name)!r}"
            )
    return mappings


def lean_constructor_name_from_key(key: str) -> str:
    parts = key.split("_")
    if not parts or any(not part for part in parts):
        fail(f"shared schema M1 op mapping has invalid key {key!r}")
    return parts[0] + "".join(part.capitalize() for part in parts[1:])


def load_m1_status_mappings(contract: dict[str, object]) -> tuple[M1StatusMapping, ...]:
    raw_mappings = contract.get("status_mappings")
    if not isinstance(raw_mappings, list) or not raw_mappings:
        fail("shared schema M1 contract is missing status_mappings")
    mappings = tuple(
        M1StatusMapping(
            key=require_string_mapping_field(entry, "key", "M1 status_mappings"),
            sv_errno=require_string_mapping_field(entry, "sv_errno", "M1 status_mappings"),
            lean_status=require_string_mapping_field(entry, "lean_status", "M1 status_mappings"),
        )
        for entry in raw_mappings
    )
    keys = tuple(mapping.key for mapping in mappings)
    if keys != M1_STATUS_KEYS:
        fail(f"shared schema M1 status mapping keys drifted: {keys!r} != {M1_STATUS_KEYS!r}")
    sv_names = [mapping.sv_errno for mapping in mappings]
    if len(sv_names) != len(set(sv_names)):
        fail(f"shared schema M1 status mappings contain duplicate errno names: {sv_names!r}")
    return mappings


def check_lean_packed_schema_contract(
    lean_source: str,
    commit_field_specs: tuple[tuple[str, int], ...],
    state_field_specs: tuple[tuple[str, int], ...],
) -> None:
    lean_commit_schema = load_lean_packed_schema(lean_source, "rtlM1CommitPackedSchema")
    if lean_commit_schema != commit_field_specs:
        fail(
            "Lean rtlM1CommitPackedSchema drifted from shared M1 commit schema: "
            f"{lean_commit_schema!r} != {commit_field_specs!r}"
        )
    lean_commit_schema_width = load_lean_packed_schema_width_theorem(
        lean_source,
        "rtlM1CommitPackedSchema_width",
        "rtlM1CommitPackedSchema",
    )
    commit_schema_width = sum(width for _name, width in commit_field_specs)
    if lean_commit_schema_width != commit_schema_width:
        fail(
            "Lean rtlM1CommitPackedSchema_width drifted from shared M1 commit schema width: "
            f"{lean_commit_schema_width} != {commit_schema_width}"
        )
    lean_state_schema = load_lean_packed_schema(lean_source, "rtlM1StateProjectionPackedSchema")
    if lean_state_schema != state_field_specs:
        fail(
            "Lean rtlM1StateProjectionPackedSchema drifted from shared M1 state schema: "
            f"{lean_state_schema!r} != {state_field_specs!r}"
        )
    lean_state_schema_width = load_lean_packed_schema_width_theorem(
        lean_source,
        "rtlM1StateProjectionPackedSchema_width",
        "rtlM1StateProjectionPackedSchema",
    )
    state_schema_width = sum(width for _name, width in state_field_specs)
    if lean_state_schema_width != state_schema_width:
        fail(
            "Lean rtlM1StateProjectionPackedSchema_width drifted from shared M1 state schema width: "
            f"{lean_state_schema_width} != {state_schema_width}"
        )
    expected_commit_projection_paths = tuple(
        (field_name, M1_SCHEMA_TO_LEAN_COMMIT_FIELDS[field_name])
        for field_name, _width in commit_field_specs
    )
    lean_commit_projection_paths = load_lean_string_pair_list(
        lean_source,
        "rtlM1CommitSchemaToLeanProjection",
    )
    if lean_commit_projection_paths != expected_commit_projection_paths:
        fail(
            "Lean rtlM1CommitSchemaToLeanProjection drifted from shared M1 commit schema: "
            f"{lean_commit_projection_paths!r} != {expected_commit_projection_paths!r}"
        )
    expected_state_field_names = tuple(field_name for field_name, _width in state_field_specs)
    actual_state_projection_field_names = tuple(
        field_name for field_name, _lean_path in M1_SCHEMA_TO_LEAN_STATE_PROJECTION_PATHS
    )
    if actual_state_projection_field_names != expected_state_field_names:
        fail(
            "internal M1 state schema-to-Lean projection paths drifted from shared M1 state schema: "
            f"{actual_state_projection_field_names!r} != {expected_state_field_names!r}"
        )
    lean_state_projection_paths = load_lean_string_pair_list(
        lean_source,
        "rtlM1StateProjectionSchemaToLeanProjection",
    )
    if lean_state_projection_paths != M1_SCHEMA_TO_LEAN_STATE_PROJECTION_PATHS:
        fail(
            "Lean rtlM1StateProjectionSchemaToLeanProjection drifted from shared M1 state schema: "
            f"{lean_state_projection_paths!r} != {M1_SCHEMA_TO_LEAN_STATE_PROJECTION_PATHS!r}"
        )


def check_lean_typed_commit_mapping(
    op_mappings: tuple[M1OpMapping, ...],
    status_mappings: tuple[M1StatusMapping, ...],
) -> None:
    source = load_lean_model_source()

    op_constructors = load_lean_inductive_constructors(source, "Op")
    missing = sorted({mapping.lean_op for mapping in op_mappings} - op_constructors)
    if missing:
        fail(f"M1 typed commit ops are missing Lean Op constructors: {missing}")

    commit_op_constructors = load_lean_inductive_constructors(source, "CommitOp")
    missing_commit_ops = sorted(
        {mapping.lean_commit_op for mapping in op_mappings} - commit_op_constructors
    )
    if missing_commit_ops:
        fail(f"M1 typed commit ops are missing Lean CommitOp constructors: {missing_commit_ops}")

    transition_constructors = load_lean_inductive_constructors(source, "TypedCommitTransition")
    missing_transitions = sorted(
        {mapping.lean_transition for mapping in op_mappings} - transition_constructors
    )
    if missing_transitions:
        fail(
            "M1 typed commit ops are missing Lean TypedCommitTransition constructors: "
            f"{missing_transitions}"
        )
    extra_transitions = sorted(
        transition_constructors - {mapping.lean_transition for mapping in op_mappings}
    )
    if extra_transitions:
        fail(
            "Lean TypedCommitTransition has constructors without RTL typed commit coverage: "
            f"{extra_transitions}"
        )

    status_constructors = load_lean_inductive_constructors(source, "CommitStatus")
    missing_statuses = sorted({mapping.lean_status for mapping in status_mappings} - status_constructors)
    if missing_statuses:
        fail(f"M1 typed commit statuses are missing Lean CommitStatus constructors: {missing_statuses}")

    theorem_names = load_lean_theorems(source)
    missing_theorems = sorted(set(M1_TYPED_COMMIT_LEAN_THEOREMS) - theorem_names)
    if missing_theorems:
        fail(f"M1 typed commit bridge theorems are missing from Lean: {missing_theorems}")

    commit_record_fields = load_lean_structure_fields(source, "CommitRecord")
    if commit_record_fields != M1_LEAN_COMMIT_RECORD_FIELDS:
        fail(
            "Lean CommitRecord fields drifted: "
            f"{commit_record_fields!r} != {M1_LEAN_COMMIT_RECORD_FIELDS!r}"
        )
    rtl_commit_projection_fields = load_lean_structure_fields(source, "RtlM1CommitProjection")
    if rtl_commit_projection_fields != M1_LEAN_COMMIT_RECORD_FIELDS:
        fail(
            "Lean RtlM1CommitProjection fields drifted from CommitRecord: "
            f"{rtl_commit_projection_fields!r} != {M1_LEAN_COMMIT_RECORD_FIELDS!r}"
        )
    rtl_state_projection_fields = load_lean_structure_fields(source, "RtlM1StateProjection")
    if rtl_state_projection_fields != M1_LEAN_RTL_STATE_PROJECTION_FIELDS:
        fail(
            "Lean RtlM1StateProjection fields drifted: "
            f"{rtl_state_projection_fields!r} != {M1_LEAN_RTL_STATE_PROJECTION_FIELDS!r}"
        )


def load_schema_contract() -> tuple[
    str,
    tuple[str, ...],
    tuple[int, ...],
    str,
    tuple[str, ...],
    tuple[int, ...],
    CommitOps,
    dict[int, str],
]:
    try:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    except OSError as exc:
        fail(f"could not read shared schema: {exc}")
    except json.JSONDecodeError as exc:
        fail(f"could not parse shared schema: {exc}")

    contract = schema.get("m1_typed_commit_contract", {})
    if not isinstance(contract, dict):
        fail("shared schema is missing m1_typed_commit_contract")
    if contract.get("stage") != "m1_typed_cap_commit_transition_mirror":
        fail("shared schema M1 typed commit contract has unexpected stage")
    op_mappings = load_m1_op_mappings(contract)
    status_mappings = load_m1_status_mappings(contract)
    check_lean_typed_commit_mapping(op_mappings, status_mappings)
    record_name = contract.get("record_name")
    state_record_name = contract.get("state_record_name")
    record_type = contract.get("record")
    state_record_type = contract.get("state_record")
    op_enum = contract.get("op_enum")
    if record_name != "m1_cap_commit":
        fail(f"shared schema M1 record_name drifted: {record_name!r}")
    if state_record_name != "m1_state_projection":
        fail(f"shared schema M1 state_record_name drifted: {state_record_name!r}")
    fields = schema.get("records", {}).get(record_type)
    if not isinstance(fields, list):
        fail(f"shared schema is missing M1 record {record_type!r}")
    commit_field_specs = tuple(parse_schema_field(entry) for entry in fields)
    expected_fields = tuple(name for name, _width in commit_field_specs)
    expected_widths = tuple(width for _name, width in commit_field_specs)
    if expected_fields != COMMIT_FIELDS:
        fail(f"M1 typed commit schema fields drifted: {expected_fields!r} != {COMMIT_FIELDS!r}")
    state_fields = schema.get("records", {}).get(state_record_type)
    if not isinstance(state_fields, list):
        fail(f"shared schema is missing M1 state projection record {state_record_type!r}")
    state_field_specs = tuple(parse_schema_field(entry) for entry in state_fields)
    expected_state_fields = tuple(name for name, _width in state_field_specs)
    expected_state_widths = tuple(width for _name, width in state_field_specs)
    if expected_state_fields != STATE_PROJECTION_FIELDS:
        fail(
            "M1 state projection schema fields drifted: "
            f"{expected_state_fields!r} != {STATE_PROJECTION_FIELDS!r}"
        )
    check_rtl_packed_typedefs_match_schema(commit_field_specs, state_field_specs)
    check_rtl_state_projection_boundary(expected_fields, expected_state_fields)
    mapped_lean_state_fields = lean_state_projection_fields_from_schema(expected_state_fields)
    if mapped_lean_state_fields != M1_LEAN_RTL_STATE_PROJECTION_FIELDS:
        fail(
            "M1 state projection schema-to-Lean RtlM1StateProjection mapping drifted: "
            f"{mapped_lean_state_fields!r} != {M1_LEAN_RTL_STATE_PROJECTION_FIELDS!r}"
        )
    mapped_lean_fields = tuple(M1_SCHEMA_TO_LEAN_COMMIT_FIELDS[field] for field in expected_fields)
    if mapped_lean_fields != M1_LEAN_COMMIT_RECORD_FIELDS:
        fail(
            "M1 typed commit schema-to-Lean CommitRecord field mapping drifted: "
            f"{mapped_lean_fields!r} != {M1_LEAN_COMMIT_RECORD_FIELDS!r}"
        )
    check_lean_packed_schema_contract(load_lean_model_source(), commit_field_specs, state_field_specs)
    enum_entries = schema.get("enums", {}).get(op_enum)
    if not isinstance(enum_entries, list):
        fail(f"shared schema is missing M1 op enum {op_enum!r}")
    enum_values: dict[str, int] = {}
    for entry in enum_entries:
        name, raw_value = entry.split("=", 1)
        enum_values[name] = parse_sv_int(raw_value)
    if len(enum_values.values()) != len(set(enum_values.values())):
        fail(f"M1 op enum contains duplicate values: {enum_values!r}")
    missing = sorted({mapping.sv for mapping in op_mappings} - set(enum_values))
    if missing:
        fail(f"M1 op enum is missing values: {missing}")
    errno_entries = schema.get("enums", {}).get("lnp64_errno_e")
    if not isinstance(errno_entries, list):
        fail("shared schema is missing errno enum lnp64_errno_e")
    errno_values: dict[str, int] = {}
    for entry in errno_entries:
        name, raw_value = entry.split("=", 1)
        errno_values[name] = parse_sv_int(raw_value)
    missing_status_values = sorted({mapping.sv_errno for mapping in status_mappings} - set(errno_values))
    if missing_status_values:
        fail(f"M1 status enum mapping is missing errno values: {missing_status_values}")
    expected_status_values = {
        "ok": ERR_OK,
        "eperm": ERR_EPERM,
        "eagain": ERR_EAGAIN,
        "erevoked": ERR_EREVOKED,
    }
    for mapping in status_mappings:
        if errno_values[mapping.sv_errno] != expected_status_values[mapping.key]:
            fail(
                f"M1 status {mapping.key} value drifted: "
                f"{errno_values[mapping.sv_errno]} != {expected_status_values[mapping.key]}"
            )
    ops = CommitOps(**{
        mapping.key: enum_values[mapping.sv]
        for mapping in op_mappings
    })
    transition_names = {
        enum_values[mapping.sv]: f"TypedCommitTransition.{mapping.lean_transition}"
        for mapping in op_mappings
    }
    if set(transition_names) != ops.valid_ops:
        fail(
            "M1 schema-owned Lean transition map does not cover the valid RTL op set: "
            f"{sorted(transition_names)!r} != {sorted(ops.valid_ops)!r}"
        )
    return (
        record_name,
        expected_fields,
        expected_widths,
        state_record_name,
        expected_state_fields,
        expected_state_widths,
        ops,
        transition_names,
    )


def run_m1_gate() -> str:
    log_path = Path(os.environ.get("LNP64_M1_TYPED_COMMIT_LOG", DEFAULT_M1_TRACE_LOG))
    if os.environ.get("LNP64_M1_TYPED_COMMIT_USE_EXISTING") == "1":
        try:
            return log_path.read_text(encoding="utf-8")
        except OSError as exc:
            fail(f"missing existing M1 typed commit log {log_path}: {exc}")

    env = os.environ.copy()
    env.setdefault("LNP64_COSIM_SEEDS", DEFAULT_TYPED_COMMIT_SEEDS)
    proc = subprocess.run(
        ["bash", "scripts/run_rtl_m1.sh"],
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if proc.returncode != 0:
        print(proc.stdout, end="")
        fail(f"scripts/run_rtl_m1.sh exited with {proc.returncode}")
    try:
        log_path.write_text(proc.stdout, encoding="utf-8")
    except OSError as exc:
        fail(f"could not write M1 typed commit log {log_path}: {exc}")
    return proc.stdout


def parse_records(
    output: str,
    expected_record_name: str,
    expected_fields: tuple[str, ...],
) -> list[dict[str, int | str]]:
    records: list[dict[str, int | str]] = []
    for line in output.splitlines():
        if not line.startswith("TTRACE_M1 "):
            continue
        payload = line.removeprefix("TTRACE_M1 ")
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid JSON record {payload!r}: {exc}")
        if record.get("record") != expected_record_name:
            fail(f"unexpected record type {record.get('record')!r}")
        actual_fields = tuple(key for key in record if key != "record")
        if actual_fields != expected_fields:
            fail(f"M1 typed commit fields drifted: {actual_fields!r} != {expected_fields!r}")
        for field in expected_fields:
            require_int(record, field)
        records.append(record)
    if not records:
        fail("no TTRACE_M1 records emitted")
    return records


def parse_state_projection_records(
    output: str,
    prefix: str,
    expected_record_name: str,
    expected_fields: tuple[str, ...],
    label: str,
) -> list[dict[str, int | str]]:
    records: list[dict[str, int | str]] = []
    for line in output.splitlines():
        if not line.startswith(prefix):
            continue
        payload = line.removeprefix(prefix)
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid M1 {label} projection record {payload!r}: {exc}")
        if record.get("record") != expected_record_name:
            fail(f"unexpected M1 {label} projection record type {record.get('record')!r}")
        actual_fields = tuple(key for key in record if key != "record")
        if actual_fields != expected_fields:
            fail(f"M1 {label} projection fields drifted: {actual_fields!r} != {expected_fields!r}")
        for field in expected_fields:
            require_int(record, field)
        records.append(record)
    if not records:
        fail(f"no {prefix.strip()} records emitted")
    return records


def parse_bit_records(
    output: str,
    prefix: str,
    expected_record_name: str,
    expected_width: int,
) -> list[str]:
    records: list[str] = []
    for line in output.splitlines():
        if not line.startswith(prefix):
            continue
        payload = line.removeprefix(prefix)
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid packed bit record {payload!r}: {exc}")
        if record.get("record") != expected_record_name:
            fail(f"unexpected packed bit record type {record.get('record')!r}")
        width = record.get("width")
        if width != expected_width:
            fail(
                f"packed bit record {expected_record_name} width drifted from schema: "
                f"{width!r} != {expected_width}"
            )
        bits = record.get("bits")
        if not isinstance(bits, str) or not re.fullmatch(r"[0-9a-fA-F]+", bits):
            fail(f"packed bit record {expected_record_name} has invalid bits {bits!r}")
        records.append(bits)
    if not records:
        fail(f"no {prefix.strip()} records emitted")
    return records


def decode_packed_bits(
    bits: str,
    expected_fields: tuple[str, ...],
    expected_widths: tuple[int, ...],
) -> dict[str, int]:
    total_width = sum(expected_widths)
    raw = int(bits, 16)
    if raw >= (1 << total_width):
        fail(f"packed bit record is wider than schema width {total_width}: 0x{bits}")
    decoded: dict[str, int] = {}
    shift = total_width
    for field, width in zip(expected_fields, expected_widths, strict=True):
        shift -= width
        decoded[field] = (raw >> shift) & ((1 << width) - 1)
    if shift != 0:
        fail("internal packed bit decoder did not consume all schema bits")
    return decoded


def check_packed_bits_match_records(
    records: list[dict[str, int | str]],
    bit_records: list[str],
    expected_fields: tuple[str, ...],
    expected_widths: tuple[int, ...],
    label: str,
) -> None:
    if len(bit_records) != len(records):
        fail(f"{label} packed bit count {len(bit_records)} != field record count {len(records)}")
    for index, (record, bits) in enumerate(zip(records, bit_records, strict=True)):
        decoded = decode_packed_bits(bits, expected_fields, expected_widths)
        for field in expected_fields:
            actual = require_int(record, field)
            if decoded[field] != actual:
                fail(
                    f"{label} packed bit decode drift at record {index} field {field}: "
                    f"{decoded[field]} != {actual}"
                )


def split_runs(records: list[dict[str, int | str]], ops: CommitOps) -> list[list[dict[str, int | str]]]:
    runs: list[list[dict[str, int | str]]] = []
    current: list[dict[str, int | str]] = []
    for record in records:
        if record.get("op") in (ops.cap_dup, ops.cap_dup_denied) and current:
            runs.append(current)
            current = []
        current.append(record)
    if current:
        runs.append(current)
    return runs


def require_int(record: dict[str, int | str], key: str) -> int:
    value = record.get(key)
    if not isinstance(value, int):
        fail(f"record op={record.get('op')} has non-integer {key}: {value!r}")
    return value


def cap_from_record(record: dict[str, int | str]) -> Cap:
    return Cap(
        object_id=require_int(record, "object_id"),
        object_gen=require_int(record, "object_gen"),
        fdr_gen=require_int(record, "fdr_gen"),
        domain_id=require_int(record, "domain_id"),
        domain_gen=require_int(record, "domain_gen"),
        rights_mask=require_int(record, "rights_mask"),
        lineage_epoch=require_int(record, "lineage_epoch"),
        sealed=require_int(record, "sealed"),
    )


def rights_subset(child: int, parent: int) -> bool:
    return (child & ~parent) == 0


def cap_lineage_valid(state: M1State, cap: Cap) -> bool:
    return (
        cap.object_id in (state.root_cap.object_id, 2)
        and cap.domain_id in (1, 2)
        and cap.domain_gen == 1
        and cap.lineage_epoch == state.root_cap.lineage_epoch
        and cap.sealed == 0
        and rights_subset(cap.rights_mask, state.root_cap.rights_mask)
    )


def cap_generation_live(state: M1State, cap: Cap) -> bool:
    if cap.object_id == state.root_cap.object_id:
        return cap.fdr_gen == state.object_gen
    return state.created_object_created and cap.object_id == 2 and cap.fdr_gen == state.created_object_gen


def cap_currently_authorizes(state: M1State, cap: Cap) -> bool:
    return cap_lineage_valid(state, cap) and cap_generation_live(state, cap)


def root_cap_currently_authorizes(state: M1State) -> bool:
    return cap_currently_authorizes(state, state.root_cap) and state.root_cap.domain_id == 1


def can_root_duplicate(state: M1State) -> bool:
    return (
        root_cap_currently_authorizes(state)
        and (state.root_cap.rights_mask & RIGHT_DUP) != 0
        and (state.root_cap.rights_mask & RIGHT_PULL) != 0
    )


def can_root_push(state: M1State) -> bool:
    return root_cap_currently_authorizes(state) and (state.root_cap.rights_mask & RIGHT_PUSH) != 0


def can_root_mint(state: M1State) -> bool:
    return root_cap_currently_authorizes(state) and (state.root_cap.rights_mask & RIGHT_MINT) != 0


def can_consumer_pull_from_main_object(state: M1State, cap: Cap) -> bool:
    return (
        cap_currently_authorizes(state, cap)
        and cap.object_id == state.root_cap.object_id
        and cap.fdr_gen == state.object_gen
        and cap.domain_id == 2
        and cap.sealed == 0
        and (cap.rights_mask & RIGHT_PULL) != 0
    )


def require_cap_equal(left: Cap, right: Cap, context: str) -> None:
    if left != right:
        fail(f"{context}: cap mismatch {left!r} != {right!r}")


def lean_transition_constructor(op: int, transition_names: dict[int, str]) -> str:
    transition = transition_names.get(op)
    if transition is None:
        fail(f"unsupported M1 commit op {op}")
    return transition


def cap_projection(prefix: str, cap: Cap | None) -> dict[str, int]:
    if cap is None:
        return {
            f"{prefix}_object_id": 0,
            f"{prefix}_generation": 0,
            f"{prefix}_domain_id": 0,
            f"{prefix}_lineage_epoch": 0,
            f"{prefix}_sealed": 0,
            f"{prefix}_rights": 0,
        }
    return {
        f"{prefix}_object_id": cap.object_id,
        f"{prefix}_generation": cap.fdr_gen,
        f"{prefix}_domain_id": cap.domain_id,
        f"{prefix}_lineage_epoch": cap.lineage_epoch,
        f"{prefix}_sealed": cap.sealed,
        f"{prefix}_rights": cap.rights_mask,
    }


def projection_from_state(state: M1State, op: int, status: int) -> dict[str, int | str]:
    consumer_cap = state.consumer_cap
    if consumer_cap is None:
        consumer_cap = Cap(
            object_id=state.root_cap.object_id,
            object_gen=state.object_gen,
            fdr_gen=0,
            domain_id=2,
            domain_gen=1,
            rights_mask=0,
            lineage_epoch=state.root_cap.lineage_epoch,
            sealed=0,
        )
    return {
        "record": "m1_state_projection",
        "op": op,
        "status": status,
        "object_gen": state.object_gen,
        "created_object_created": int(state.created_object_created),
        "created_object_gen": state.created_object_gen,
        **cap_projection("root", state.root_cap),
        **cap_projection("consumer", consumer_cap),
        "sent_valid": int(state.sent_cap is not None),
        **cap_projection("sent", state.sent_cap),
        "minted_valid": int(state.minted_cap is not None),
        **cap_projection("minted", state.minted_cap),
        "wake_pending": int(state.wake_pending),
        "transfer_valid": int(state.transfer_valid),
        "stale_rejected": int(state.stale_rejected),
        "revoked_rejected": int(state.revoked_rejected),
        "failed_no_authority": int(state.failed_no_authority),
        "full_was_explicit": int(state.full_was_explicit),
        "has_revoked_generation": int(state.revoked_gen is not None),
        "revoked_generation": state.revoked_gen or 0,
    }


def check_state_projection(
    expected: dict[str, int | str],
    actual: dict[str, int | str],
    context: str,
) -> None:
    expected_fields = ("record",) + STATE_PROJECTION_FIELDS
    actual_fields = tuple(actual)
    if actual_fields != expected_fields:
        fail(f"{context}: state projection fields drifted: {actual_fields!r} != {expected_fields!r}")
    for field in expected_fields:
        if actual.get(field) != expected.get(field):
            fail(f"{context}: state projection field {field} {actual.get(field)!r} != {expected.get(field)!r}")


def check_authority_projection_slots_unchanged(
    expected_pre: dict[str, int | str],
    actual_post: dict[str, int | str],
    context: str,
) -> None:
    for field in AUTHORITY_STATE_PROJECTION_FIELDS:
        if actual_post.get(field) != expected_pre.get(field):
            fail(
                f"{context}: non-OK commit changed authority projection field {field}: "
                f"{actual_post.get(field)!r} != {expected_pre.get(field)!r}"
            )


def check_projection_cap_matches_commit(
    prefix: str,
    commit_record: dict[str, int | str],
    post_state_record: dict[str, int | str],
    context: str,
) -> None:
    field_pairs = (
        (f"{prefix}_object_id", "object_id"),
        (f"{prefix}_generation", "fdr_gen"),
        (f"{prefix}_domain_id", "domain_id"),
        (f"{prefix}_lineage_epoch", "lineage_epoch"),
        (f"{prefix}_sealed", "sealed"),
        (f"{prefix}_rights", "rights_mask"),
    )
    for projection_field, commit_field in field_pairs:
        if post_state_record.get(projection_field) != commit_record.get(commit_field):
            fail(
                f"{context}: post {prefix} projection field {projection_field} "
                f"{post_state_record.get(projection_field)!r} != commit {commit_field} "
                f"{commit_record.get(commit_field)!r}"
            )


def check_rtl_refinement_postcondition(
    expected_pre_projection: dict[str, int | str],
    commit_record: dict[str, int | str],
    post_state_record: dict[str, int | str],
    context: str,
    ops: CommitOps,
) -> None:
    op = require_int(commit_record, "op")
    status = require_int(commit_record, "status")
    if op == ops.cap_dup:
        check_projection_cap_matches_commit("consumer", commit_record, post_state_record, context)
        return
    if op == ops.cap_send:
        if post_state_record.get("sent_valid") != 1:
            fail(f"{context}: capSend postcondition did not publish sent cap")
        check_projection_cap_matches_commit("sent", commit_record, post_state_record, context)
        return
    if op == ops.cap_recv:
        check_projection_cap_matches_commit("consumer", commit_record, post_state_record, context)
        if post_state_record.get("sent_valid") != 0:
            fail(f"{context}: capRecv postcondition did not clear sent cap")
        return
    if op == ops.cap_revoke:
        object_gen = require_int(commit_record, "object_gen")
        fdr_gen = require_int(commit_record, "fdr_gen")
        if post_state_record.get("object_gen") != object_gen:
            fail(f"{context}: capRevoke postcondition object_gen did not match commit")
        if post_state_record.get("root_generation") != object_gen:
            fail(f"{context}: capRevoke postcondition root generation did not match live generation")
        if post_state_record.get("revoked_generation") != fdr_gen:
            fail(f"{context}: capRevoke postcondition revoked_generation did not match old generation")
        if post_state_record.get("has_revoked_generation") != 1:
            fail(f"{context}: capRevoke postcondition did not publish revoked-generation witness")
        return
    if op == ops.push:
        check_projection_cap_matches_commit("root", commit_record, post_state_record, context)
        if post_state_record.get("wake_pending") != 1:
            fail(f"{context}: push postcondition did not set wake_pending")
        return
    if op == ops.pull:
        check_projection_cap_matches_commit("consumer", commit_record, post_state_record, context)
        if post_state_record.get("wake_pending") != 0:
            fail(f"{context}: pull postcondition did not clear wake_pending")
        return
    if op == ops.reject_full:
        if status != ERR_EAGAIN:
            fail(f"{context}: rejectFull postcondition status {status!r} != EAGAIN")
        if post_state_record.get("full_was_explicit") != 1:
            fail(f"{context}: rejectFull postcondition did not set full_was_explicit")
        check_authority_projection_slots_unchanged(expected_pre_projection, post_state_record, context)
        return
    if op == ops.reject_stale:
        if status != ERR_EREVOKED:
            fail(f"{context}: rejectStale postcondition status {status!r} != EREVOKED")
        if post_state_record.get("stale_rejected") != 1:
            fail(f"{context}: rejectStale postcondition did not set stale_rejected")
        check_authority_projection_slots_unchanged(expected_pre_projection, post_state_record, context)
        return
    if op == ops.cap_dup_denied:
        if status != ERR_EPERM:
            fail(f"{context}: capDupDenied postcondition status {status!r} != EPERM")
        if post_state_record.get("failed_no_authority") != 1:
            fail(f"{context}: capDupDenied postcondition did not set failed_no_authority")
        check_authority_projection_slots_unchanged(expected_pre_projection, post_state_record, context)
        return
    if op == ops.object_create:
        if post_state_record.get("minted_valid") != 1:
            fail(f"{context}: objectCreate postcondition did not mint a cap")
        check_projection_cap_matches_commit("minted", commit_record, post_state_record, context)
        if post_state_record.get("created_object_created") != 1:
            fail(f"{context}: objectCreate postcondition did not mark created object")


def check_rtl_refinement_step(
    state: M1State,
    pre_state_record: dict[str, int | str],
    commit_record: dict[str, int | str],
    post_state_record: dict[str, int | str],
    run_index: int,
    ops: CommitOps,
    transition_names: dict[int, str],
) -> None:
    """Check the executable mirror of Lean RtlM1RefinementStep.

    Every RTL commit must carry an emitted pre-state projection sampled from the
    real RTL boundary before the commit, plus the post-state projection emitted
    after the commit. Both records use the schema-owned packed state format.
    """
    op = require_int(commit_record, "op")
    status = require_int(commit_record, "status")
    transition = lean_transition_constructor(op, transition_names)
    expected_pre_projection = projection_from_state(state, op, status)
    check_state_projection(
        expected_pre_projection,
        pre_state_record,
        f"run {run_index} {transition} RtlM1RefinementStep pre-state projection",
    )
    apply_commit(state, commit_record, run_index, ops, transition_names)
    if status != ERR_OK:
        check_authority_projection_slots_unchanged(
            expected_pre_projection,
            post_state_record,
            f"run {run_index} {transition} RtlM1RefinementStep",
        )
    check_rtl_refinement_postcondition(
        expected_pre_projection,
        commit_record,
        post_state_record,
        f"run {run_index} {transition} RtlM1RefinementStep",
        ops,
    )
    check_state_projection(
        projection_from_state(state, op, status),
        post_state_record,
        f"run {run_index} {transition} RtlM1RefinementStep post-state projection",
    )


def initial_state(first_record: dict[str, int | str], ops: CommitOps) -> M1State:
    initial_gen = require_int(first_record, "object_gen")
    root_rights = ROOT_RIGHTS
    if require_int(first_record, "op") == ops.cap_dup_denied:
        root_rights = require_int(first_record, "rights_mask")
    root_cap = Cap(
        object_id=1,
        object_gen=initial_gen,
        fdr_gen=initial_gen,
        domain_id=1,
        domain_gen=1,
        rights_mask=root_rights,
        lineage_epoch=1,
        sealed=0,
    )
    return M1State(object_gen=initial_gen, root_cap=root_cap)


def apply_commit(
    state: M1State,
    record: dict[str, int | str],
    run_index: int,
    ops: CommitOps,
    transition_names: dict[int, str],
) -> None:
    op = require_int(record, "op")
    cap = cap_from_record(record)
    status = require_int(record, "status")
    transition = lean_transition_constructor(op, transition_names)

    if op == ops.cap_dup:
        if not can_root_duplicate(state):
            fail(f"run {run_index} {transition} root duplicate precondition failed")
        expected = Cap(
            object_id=state.root_cap.object_id,
            object_gen=state.object_gen,
            fdr_gen=state.object_gen,
            domain_id=2,
            domain_gen=1,
            rights_mask=RIGHT_PULL,
            lineage_epoch=state.root_cap.lineage_epoch,
            sealed=0,
        )
        require_cap_equal(cap, expected, f"run {run_index} {transition} commit projection")
        if status != ERR_OK:
            fail(f"run {run_index} {transition} emitted non-OK status")
        state.consumer_cap = cap
        return

    if op == ops.cap_send:
        if state.consumer_cap is None:
            fail(f"run {run_index} {transition} before capDup")
        if not cap_currently_authorizes(state, state.consumer_cap):
            fail(f"run {run_index} {transition} consumer cap does not currently authorize transfer")
        require_cap_equal(cap, state.consumer_cap, f"run {run_index} {transition} commit projection")
        if status != ERR_OK:
            fail(f"run {run_index} {transition} emitted non-OK status")
        state.sent_cap = state.consumer_cap
        state.transfer_valid = True
        return

    if op == ops.cap_recv:
        if state.sent_cap is None:
            fail(f"run {run_index} {transition} with empty transfer slot")
        if not cap_currently_authorizes(state, state.sent_cap):
            fail(f"run {run_index} {transition} sent cap does not currently authorize transfer")
        require_cap_equal(cap, state.sent_cap, f"run {run_index} {transition} commit projection")
        if status != ERR_OK:
            fail(f"run {run_index} {transition} emitted non-OK status")
        state.consumer_cap = state.sent_cap
        state.sent_cap = None
        state.transfer_valid = True
        return

    if op == ops.push:
        if not can_root_push(state):
            fail(f"run {run_index} {transition} root push precondition failed")
        require_cap_equal(cap, state.root_cap, f"run {run_index} {transition} commit projection")
        if status != ERR_OK:
            fail(f"run {run_index} {transition} emitted non-OK status")
        state.queue_full = True
        state.wake_pending = True
        return

    if op == ops.pull:
        if state.consumer_cap is None:
            fail(f"run {run_index} {transition} before consumer cap exists")
        if not can_consumer_pull_from_main_object(state, state.consumer_cap):
            fail(f"run {run_index} {transition} consumer pull precondition failed")
        if not state.queue_full:
            fail(f"run {run_index} {transition} while queue is empty")
        require_cap_equal(cap, state.consumer_cap, f"run {run_index} {transition} commit projection")
        if status != ERR_OK:
            fail(f"run {run_index} {transition} emitted non-OK status")
        state.queue_full = False
        state.wake_pending = False
        return

    if op == ops.reject_full:
        # The RTL has an untyped queue_refill micro-step between pull and
        # rejectFull. Model it here so rejectFull checks the real precondition.
        if not state.queue_full:
            state.queue_full = True
        if not state.queue_full:
            fail(f"run {run_index} {transition} without a full queue")
        require_cap_equal(cap, state.root_cap, f"run {run_index} {transition} commit projection")
        if status != ERR_EAGAIN:
            fail(f"run {run_index} {transition} did not fail with EAGAIN")
        state.full_was_explicit = True
        return

    if op == ops.object_create:
        if not can_root_mint(state):
            fail(f"run {run_index} {transition} root mint precondition failed")
        expected = Cap(
            object_id=2,
            object_gen=state.created_object_gen,
            fdr_gen=state.created_object_gen,
            domain_id=1,
            domain_gen=1,
            rights_mask=ROOT_RIGHTS,
            lineage_epoch=state.root_cap.lineage_epoch,
            sealed=0,
        )
        require_cap_equal(cap, expected, f"run {run_index} {transition} commit projection")
        if status != ERR_OK:
            fail(f"run {run_index} {transition} emitted non-OK status")
        state.created_object_created = True
        state.minted_cap = cap
        if not cap_currently_authorizes(state, state.minted_cap):
            fail(f"run {run_index} {transition} minted cap does not authorize created object")
        return

    if op == ops.cap_revoke:
        if not root_cap_currently_authorizes(state):
            fail(f"run {run_index} {transition} root authority precondition failed")
        old_gen = state.object_gen
        expected = Cap(
            object_id=state.root_cap.object_id,
            object_gen=old_gen + 1,
            fdr_gen=old_gen,
            domain_id=1,
            domain_gen=1,
            rights_mask=ROOT_RIGHTS,
            lineage_epoch=state.root_cap.lineage_epoch,
            sealed=0,
        )
        require_cap_equal(cap, expected, f"run {run_index} {transition} commit projection")
        if status != ERR_OK:
            fail(f"run {run_index} {transition} emitted non-OK status")
        state.object_gen = old_gen + 1
        state.revoked_gen = old_gen
        state.revoked_rejected = True
        state.stale_rejected = True
        state.root_cap = Cap(
            object_id=state.root_cap.object_id,
            object_gen=state.object_gen,
            fdr_gen=state.object_gen,
            domain_id=1,
            domain_gen=1,
            rights_mask=ROOT_RIGHTS,
            lineage_epoch=state.root_cap.lineage_epoch,
            sealed=0,
        )
        return

    if op == ops.reject_stale:
        if state.consumer_cap is None:
            fail(f"run {run_index} {transition} before consumer cap exists")
        expected = state.consumer_cap
        expected_live_projection = Cap(
            object_id=expected.object_id,
            object_gen=state.object_gen,
            fdr_gen=expected.fdr_gen,
            domain_id=expected.domain_id,
            domain_gen=expected.domain_gen,
            rights_mask=expected.rights_mask,
            lineage_epoch=expected.lineage_epoch,
            sealed=expected.sealed,
        )
        require_cap_equal(cap, expected_live_projection, f"run {run_index} {transition} commit projection")
        if status != ERR_EREVOKED:
            fail(f"run {run_index} {transition} did not fail with EREVOKED")
        if cap_currently_authorizes(state, state.consumer_cap):
            fail(f"run {run_index} {transition} stale consumer cap still authorizes work")
        if state.revoked_gen is None or state.consumer_cap.fdr_gen != state.revoked_gen:
            fail(f"run {run_index} {transition} stale cap does not point at the revoked generation")
        state.stale_rejected = True
        return

    if op == ops.cap_dup_denied:
        if status != ERR_EPERM:
            fail(f"run {run_index} {transition} did not fail with EPERM")
        if state.root_cap.rights_mask & RIGHT_DUP:
            fail(f"run {run_index} {transition} root duplicate precondition failed")
        require_cap_equal(cap, state.root_cap, f"run {run_index} {transition} commit projection")
        state.failed_no_authority = True
        return

    fail(f"run {run_index} unsupported op {op}")


def check_common(record: dict[str, int | str], ops: CommitOps) -> None:
    if record.get("record") != "m1_cap_commit":
        fail(f"unexpected record type {record.get('record')!r}")
    op = require_int(record, "op")
    if op not in ops.valid_ops:
        fail(f"unknown M1 commit op {op}")
    if require_int(record, "object_id") not in (1, 2):
        fail(f"op {op} names unexpected object_id")
    if require_int(record, "domain_id") not in (1, 2):
        fail(f"op {op} names unknown domain")
    if require_int(record, "domain_gen") != 1:
        fail(f"op {op} uses unexpected domain generation")
    if require_int(record, "lineage_epoch") != 1:
        fail(f"op {op} uses unexpected lineage epoch")
    if require_int(record, "sealed") != 0:
        fail(f"op {op} emitted a sealed authority-bearing cap")
    rights_mask = require_int(record, "rights_mask")
    if rights_mask & ~ROOT_RIGHTS:
        fail(f"op {op} amplifies rights beyond root mask")


def check_run(
    run: list[dict[str, int | str]],
    pre_state_run: list[dict[str, int | str]],
    state_run: list[dict[str, int | str]],
    index: int,
    ops: CommitOps,
    transition_names: dict[int, str],
) -> None:
    sequence = [require_int(record, "op") for record in run]
    pre_state_sequence = [require_int(record, "op") for record in pre_state_run]
    state_sequence = [require_int(record, "op") for record in state_run]
    if pre_state_sequence != sequence:
        fail(f"run {index} pre-state projection op sequence {pre_state_sequence} != commit sequence {sequence}")
    if state_sequence != sequence:
        fail(f"run {index} state projection op sequence {state_sequence} != commit sequence {sequence}")
    if sequence == ops.denied_sequence:
        check_denied_run(run, pre_state_run, state_run, index, ops, transition_names)
        return
    if sequence != ops.expected_sequence:
        fail(f"run {index} op sequence {sequence} != {ops.expected_sequence} or {ops.denied_sequence}")
    if len(pre_state_run) != len(run):
        fail(f"run {index} pre-state projection count {len(pre_state_run)} != commit count {len(run)}")
    if len(state_run) != len(run):
        fail(f"run {index} state projection count {len(state_run)} != commit count {len(run)}")

    for record in run:
        check_common(record, ops)

    state = initial_state(run[0], ops)
    for record, pre_state_record, state_record in zip(run, pre_state_run, state_run, strict=True):
        check_rtl_refinement_step(
            state,
            pre_state_record,
            record,
            state_record,
            index,
            ops,
            transition_names,
        )
    if state.sent_cap is not None:
        fail(f"run {index} ended with an undelivered transferred cap")
    if state.consumer_cap is None:
        fail(f"run {index} ended without a consumer capability")
    if state.minted_cap is None:
        fail(f"run {index} ended without a minted object capability")
    if not state.transfer_valid:
        fail(f"run {index} never completed a valid transfer")
    if not state.stale_rejected:
        fail(f"run {index} did not reject the stale generation")
    if state.revoked_gen is None or state.consumer_cap.fdr_gen != state.revoked_gen:
        fail(f"run {index} did not preserve the revoked-generation witness")

    by_op = {require_int(record, "op"): record for record in run}
    cap_dup = by_op[ops.cap_dup]
    cap_send = by_op[ops.cap_send]
    cap_recv = by_op[ops.cap_recv]
    push = by_op[ops.push]
    pull = by_op[ops.pull]
    reject_full = by_op[ops.reject_full]
    object_create = by_op[ops.object_create]
    revoke = by_op[ops.cap_revoke]
    reject_stale = by_op[ops.reject_stale]

    for op, record in ((ops.cap_dup, cap_dup), (ops.cap_send, cap_send), (ops.cap_recv, cap_recv)):
        if require_int(record, "domain_id") != 2:
            fail(f"run {index} op {op} did not transfer to consumer domain")
        if require_int(record, "rights_mask") != RIGHT_PULL:
            fail(f"run {index} op {op} did not narrow to pull-only rights")
        if require_int(record, "status") != ERR_OK:
            fail(f"run {index} op {op} did not commit successfully")

    for field in CAP_FIELDS:
        if require_int(cap_dup, field) != require_int(cap_send, field):
            fail(f"run {index} capSend changed transferred cap field {field}")
        if require_int(cap_send, field) != require_int(cap_recv, field):
            fail(f"run {index} capRecv changed transferred cap field {field}")

    for op, record in ((ops.push, push), (ops.reject_full, reject_full), (ops.cap_revoke, revoke)):
        if require_int(record, "domain_id") != 1:
            fail(f"run {index} op {op} was not rooted in the owner domain")
        if require_int(record, "rights_mask") != ROOT_RIGHTS:
            fail(f"run {index} op {op} does not use root authority")

    if require_int(push, "status") != ERR_OK:
        fail(f"run {index} push did not commit successfully")
    if require_int(pull, "domain_id") != 2 or require_int(pull, "rights_mask") != RIGHT_PULL:
        fail(f"run {index} pull did not use the consumer pull capability")
    if require_int(pull, "status") != ERR_OK:
        fail(f"run {index} pull did not commit successfully")
    if require_int(reject_full, "status") != ERR_EAGAIN:
        fail(f"run {index} rejectFull did not report EAGAIN")

    if require_int(object_create, "object_id") != 2:
        fail(f"run {index} objectCreate did not name the created object")
    if require_int(object_create, "domain_id") != 1:
        fail(f"run {index} objectCreate was not rooted in owner domain")
    if require_int(object_create, "rights_mask") != ROOT_RIGHTS:
        fail(f"run {index} objectCreate did not mint root-bounded rights")
    if require_int(object_create, "fdr_gen") != require_int(object_create, "object_gen"):
        fail(f"run {index} objectCreate minted a stale created-object cap")
    if require_int(object_create, "status") != ERR_OK:
        fail(f"run {index} objectCreate did not commit successfully")

    old_gen = require_int(cap_dup, "object_gen")
    if require_int(revoke, "object_gen") != old_gen + 1:
        fail(f"run {index} capRevoke did not advance object generation")
    if require_int(revoke, "fdr_gen") != old_gen:
        fail(f"run {index} capRevoke did not identify the revoked generation")
    if require_int(revoke, "status") != ERR_OK:
        fail(f"run {index} capRevoke did not commit successfully")

    if require_int(reject_stale, "domain_id") != 2:
        fail(f"run {index} rejectStale did not check the consumer cap")
    if require_int(reject_stale, "rights_mask") != RIGHT_PULL:
        fail(f"run {index} rejectStale did not preserve pull-only rights")
    if require_int(reject_stale, "status") != ERR_EREVOKED:
        fail(f"run {index} rejectStale did not report EREVOKED")
    if require_int(reject_stale, "object_gen") != require_int(revoke, "object_gen"):
        fail(f"run {index} rejectStale did not check against the live generation")
    if require_int(reject_stale, "fdr_gen") == require_int(reject_stale, "object_gen"):
        fail(f"run {index} stale FDR generation was accepted as live")


def check_denied_run(
    run: list[dict[str, int | str]],
    pre_state_run: list[dict[str, int | str]],
    state_run: list[dict[str, int | str]],
    index: int,
    ops: CommitOps,
    transition_names: dict[int, str],
) -> None:
    if len(run) != 1:
        fail(f"run {index} denied path emitted more than one commit")
    if len(state_run) != 1:
        fail(f"run {index} denied path emitted more than one state projection")
    if len(pre_state_run) != 1:
        fail(f"run {index} denied path emitted more than one pre-state projection")
    record = run[0]
    check_common(record, ops)
    state = initial_state(record, ops)
    check_rtl_refinement_step(
        state,
        pre_state_run[0],
        record,
        state_run[0],
        index,
        ops,
        transition_names,
    )


def main() -> int:
    (
        record_name,
        schema_fields,
        schema_widths,
        state_record_name,
        state_schema_fields,
        state_schema_widths,
        ops,
        transition_names,
    ) = load_schema_contract()
    output = run_m1_gate()
    records = parse_records(output, record_name, schema_fields)
    bit_records = parse_bit_records(
        output,
        "TTRACE_M1_BITS ",
        "m1_cap_commit_bits",
        sum(schema_widths),
    )
    pre_state_records = parse_state_projection_records(
        output,
        "TTRACE_M1_PRE_STATE ",
        state_record_name,
        state_schema_fields,
        "pre-state",
    )
    pre_state_bit_records = parse_bit_records(
        output,
        "TTRACE_M1_PRE_STATE_BITS ",
        "m1_state_projection_bits",
        sum(state_schema_widths),
    )
    state_records = parse_state_projection_records(
        output,
        "TTRACE_M1_STATE ",
        state_record_name,
        state_schema_fields,
        "post-state",
    )
    state_bit_records = parse_bit_records(
        output,
        "TTRACE_M1_STATE_BITS ",
        "m1_state_projection_bits",
        sum(state_schema_widths),
    )
    check_packed_bits_match_records(
        records,
        bit_records,
        schema_fields,
        schema_widths,
        "M1 typed commit",
    )
    check_packed_bits_match_records(
        pre_state_records,
        pre_state_bit_records,
        state_schema_fields,
        state_schema_widths,
        "M1 pre-state projection",
    )
    check_packed_bits_match_records(
        state_records,
        state_bit_records,
        state_schema_fields,
        state_schema_widths,
        "M1 state projection",
    )
    if len(state_records) != len(records):
        fail(f"M1 state projection count {len(state_records)} != commit count {len(records)}")
    if len(pre_state_records) != len(records):
        fail(f"M1 pre-state projection count {len(pre_state_records)} != commit count {len(records)}")
    runs = split_runs(records, ops)
    pre_state_runs = split_runs(pre_state_records, ops)
    state_runs = split_runs(state_records, ops)
    if len(pre_state_runs) != len(runs):
        fail(f"M1 pre-state projection run count {len(pre_state_runs)} != commit run count {len(runs)}")
    if len(state_runs) != len(runs):
        fail(f"M1 state projection run count {len(state_runs)} != commit run count {len(runs)}")
    for index, (run, pre_state_run, state_run) in enumerate(
        zip(runs, pre_state_runs, state_runs, strict=True)
    ):
        check_run(run, pre_state_run, state_run, index, ops, transition_names)
    print(f"rtl m1 typed commit trace ok ({len(runs)} run(s))")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

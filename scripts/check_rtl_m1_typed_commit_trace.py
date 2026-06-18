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

M1_OP_ENUM_NAMES = {
    "cap_dup": "LNP64_M1_COMMIT_CAP_DUP",
    "cap_send": "LNP64_M1_COMMIT_CAP_SEND",
    "cap_recv": "LNP64_M1_COMMIT_CAP_RECV",
    "cap_revoke": "LNP64_M1_COMMIT_CAP_REVOKE",
    "reject_stale": "LNP64_M1_COMMIT_REJECT_STALE",
    "push": "LNP64_M1_COMMIT_PUSH",
    "pull": "LNP64_M1_COMMIT_PULL",
    "reject_full": "LNP64_M1_COMMIT_REJECT_FULL",
    "cap_dup_denied": "LNP64_M1_COMMIT_CAP_DUP_DENIED",
    "object_create": "LNP64_M1_COMMIT_OBJECT_CREATE",
}

M1_TYPED_COMMIT_LEAN_OPS = {
    "cap_dup": "capDup",
    "cap_send": "capSend",
    "cap_recv": "capRecv",
    "cap_revoke": "capRevoke",
    "reject_stale": "rejectStale",
    "push": "push",
    "pull": "pull",
    "reject_full": "rejectFull",
    "cap_dup_denied": "capDupDenied",
    "object_create": "objectCreate",
}

M1_TYPED_COMMIT_LEAN_COMMIT_OPS = M1_TYPED_COMMIT_LEAN_OPS
M1_TYPED_COMMIT_LEAN_TRANSITIONS = M1_TYPED_COMMIT_LEAN_OPS

M1_STATUS_ENUM_NAMES = {
    "ok": "LNP64_ERR_OK",
    "eperm": "LNP64_ERR_EPERM",
    "eagain": "LNP64_ERR_EAGAIN",
    "erevoked": "LNP64_ERR_EREVOKED",
}

M1_TYPED_COMMIT_LEAN_STATUSES = {
    "ok": "ok",
    "eperm": "eperm",
    "eagain": "eagain",
    "erevoked": "erevoked",
}

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
    "rtl_m1_refinement_step_refines_lean_step",
    "rtl_m1_refinement_step_preserves_sg_auth_invariant",
    "m1_t3_typed_commit_transition_refines_step_for_reachable",
    "m1_t3_typed_commit_transition_preserves_invariant_for_reachable",
    "m1_t3_rtl_m1_refinement_step_preserves_sg_auth_invariant_for_reachable",
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


def load_lean_model_source() -> str:
    try:
        return LEAN_M1_MODEL.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"could not read Lean M1 transition model: {exc}")


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


def load_lean_theorems(source: str) -> set[str]:
    return set(re.findall(r"(?m)^(?:theorem|lemma)\s+([A-Za-z0-9_']+)\b", source))


def check_lean_typed_commit_mapping() -> None:
    if set(M1_OP_ENUM_NAMES) != set(M1_TYPED_COMMIT_LEAN_OPS):
        fail("internal M1 RTL enum to Lean Op mapping keys drifted")
    if set(M1_OP_ENUM_NAMES) != set(M1_TYPED_COMMIT_LEAN_COMMIT_OPS):
        fail("internal M1 RTL enum to Lean CommitOp mapping keys drifted")
    if set(M1_OP_ENUM_NAMES) != set(M1_TYPED_COMMIT_LEAN_TRANSITIONS):
        fail("internal M1 RTL enum to Lean TypedCommitTransition mapping keys drifted")
    if set(M1_STATUS_ENUM_NAMES) != set(M1_TYPED_COMMIT_LEAN_STATUSES):
        fail("internal M1 RTL status to Lean CommitStatus mapping keys drifted")
    source = load_lean_model_source()

    op_constructors = load_lean_inductive_constructors(source, "Op")
    missing = sorted(set(M1_TYPED_COMMIT_LEAN_OPS.values()) - op_constructors)
    if missing:
        fail(f"M1 typed commit ops are missing Lean Op constructors: {missing}")

    commit_op_constructors = load_lean_inductive_constructors(source, "CommitOp")
    missing_commit_ops = sorted(
        set(M1_TYPED_COMMIT_LEAN_COMMIT_OPS.values()) - commit_op_constructors
    )
    if missing_commit_ops:
        fail(f"M1 typed commit ops are missing Lean CommitOp constructors: {missing_commit_ops}")

    transition_constructors = load_lean_inductive_constructors(source, "TypedCommitTransition")
    missing_transitions = sorted(
        set(M1_TYPED_COMMIT_LEAN_TRANSITIONS.values()) - transition_constructors
    )
    if missing_transitions:
        fail(
            "M1 typed commit ops are missing Lean TypedCommitTransition constructors: "
            f"{missing_transitions}"
        )
    extra_transitions = sorted(
        transition_constructors - set(M1_TYPED_COMMIT_LEAN_TRANSITIONS.values())
    )
    if extra_transitions:
        fail(
            "Lean TypedCommitTransition has constructors without RTL typed commit coverage: "
            f"{extra_transitions}"
        )

    status_constructors = load_lean_inductive_constructors(source, "CommitStatus")
    missing_statuses = sorted(set(M1_TYPED_COMMIT_LEAN_STATUSES.values()) - status_constructors)
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


def load_schema_contract() -> tuple[str, tuple[str, ...], CommitOps]:
    check_lean_typed_commit_mapping()
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
    record_name = contract.get("record_name")
    record_type = contract.get("record")
    op_enum = contract.get("op_enum")
    if record_name != "m1_cap_commit":
        fail(f"shared schema M1 record_name drifted: {record_name!r}")
    fields = schema.get("records", {}).get(record_type)
    if not isinstance(fields, list):
        fail(f"shared schema is missing M1 record {record_type!r}")
    expected_fields = tuple(entry.split(":", 1)[0] for entry in fields)
    if expected_fields != COMMIT_FIELDS:
        fail(f"M1 typed commit schema fields drifted: {expected_fields!r} != {COMMIT_FIELDS!r}")
    mapped_lean_fields = tuple(M1_SCHEMA_TO_LEAN_COMMIT_FIELDS[field] for field in expected_fields)
    if mapped_lean_fields != M1_LEAN_COMMIT_RECORD_FIELDS:
        fail(
            "M1 typed commit schema-to-Lean CommitRecord field mapping drifted: "
            f"{mapped_lean_fields!r} != {M1_LEAN_COMMIT_RECORD_FIELDS!r}"
        )
    enum_entries = schema.get("enums", {}).get(op_enum)
    if not isinstance(enum_entries, list):
        fail(f"shared schema is missing M1 op enum {op_enum!r}")
    enum_values: dict[str, int] = {}
    for entry in enum_entries:
        name, raw_value = entry.split("=", 1)
        enum_values[name] = parse_sv_int(raw_value)
    missing = sorted(set(M1_OP_ENUM_NAMES.values()) - set(enum_values))
    if missing:
        fail(f"M1 op enum is missing values: {missing}")
    errno_entries = schema.get("enums", {}).get("lnp64_errno_e")
    if not isinstance(errno_entries, list):
        fail("shared schema is missing errno enum lnp64_errno_e")
    errno_values: dict[str, int] = {}
    for entry in errno_entries:
        name, raw_value = entry.split("=", 1)
        errno_values[name] = parse_sv_int(raw_value)
    missing_status_values = sorted(set(M1_STATUS_ENUM_NAMES.values()) - set(errno_values))
    if missing_status_values:
        fail(f"M1 status enum mapping is missing errno values: {missing_status_values}")
    expected_status_values = {
        "ok": ERR_OK,
        "eperm": ERR_EPERM,
        "eagain": ERR_EAGAIN,
        "erevoked": ERR_EREVOKED,
    }
    for status_key, enum_name in M1_STATUS_ENUM_NAMES.items():
        if errno_values[enum_name] != expected_status_values[status_key]:
            fail(
                f"M1 status {status_key} value drifted: "
                f"{errno_values[enum_name]} != {expected_status_values[status_key]}"
            )
    ops = CommitOps(**{
        field: enum_values[enum_name]
        for field, enum_name in M1_OP_ENUM_NAMES.items()
    })
    return record_name, expected_fields, ops


def run_m1_gate() -> str:
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
    return root_cap_currently_authorizes(state) and (state.root_cap.rights_mask & RIGHT_DUP) != 0


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


def initial_state(first_record: dict[str, int | str]) -> M1State:
    initial_gen = require_int(first_record, "object_gen")
    root_cap = Cap(
        object_id=1,
        object_gen=initial_gen,
        fdr_gen=initial_gen,
        domain_id=1,
        domain_gen=1,
        rights_mask=ROOT_RIGHTS,
        lineage_epoch=1,
        sealed=0,
    )
    return M1State(object_gen=initial_gen, root_cap=root_cap)


def apply_commit(state: M1State, record: dict[str, int | str], run_index: int, ops: CommitOps) -> None:
    op = require_int(record, "op")
    cap = cap_from_record(record)
    status = require_int(record, "status")

    if op == ops.cap_dup:
        if not can_root_duplicate(state):
            fail(f"run {run_index} capDup root duplicate precondition failed")
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
        require_cap_equal(cap, expected, f"run {run_index} capDup post-state")
        if status != ERR_OK:
            fail(f"run {run_index} capDup failed")
        state.consumer_cap = cap
        return

    if op == ops.cap_send:
        if state.consumer_cap is None:
            fail(f"run {run_index} capSend before capDup")
        if not cap_currently_authorizes(state, state.consumer_cap):
            fail(f"run {run_index} capSend consumer cap does not currently authorize transfer")
        require_cap_equal(cap, state.consumer_cap, f"run {run_index} capSend")
        if status != ERR_OK:
            fail(f"run {run_index} capSend failed")
        state.sent_cap = state.consumer_cap
        state.transfer_valid = True
        return

    if op == ops.cap_recv:
        if state.sent_cap is None:
            fail(f"run {run_index} capRecv with empty transfer slot")
        if not cap_currently_authorizes(state, state.sent_cap):
            fail(f"run {run_index} capRecv sent cap does not currently authorize transfer")
        require_cap_equal(cap, state.sent_cap, f"run {run_index} capRecv")
        if status != ERR_OK:
            fail(f"run {run_index} capRecv failed")
        state.consumer_cap = state.sent_cap
        state.sent_cap = None
        state.transfer_valid = True
        return

    if op == ops.push:
        if not can_root_push(state):
            fail(f"run {run_index} push root push precondition failed")
        require_cap_equal(cap, state.root_cap, f"run {run_index} push")
        if status != ERR_OK:
            fail(f"run {run_index} push failed")
        state.queue_full = True
        state.wake_pending = True
        return

    if op == ops.pull:
        if state.consumer_cap is None:
            fail(f"run {run_index} pull before consumer cap exists")
        if not can_consumer_pull_from_main_object(state, state.consumer_cap):
            fail(f"run {run_index} pull consumer pull precondition failed")
        if not state.queue_full:
            fail(f"run {run_index} pull while queue is empty")
        require_cap_equal(cap, state.consumer_cap, f"run {run_index} pull")
        if status != ERR_OK:
            fail(f"run {run_index} pull failed")
        state.queue_full = False
        state.wake_pending = False
        return

    if op == ops.reject_full:
        # The RTL has an untyped queue_refill micro-step between pull and
        # rejectFull. Model it here so rejectFull checks the real precondition.
        if not state.queue_full:
            state.queue_full = True
        if not state.queue_full:
            fail(f"run {run_index} rejectFull without a full queue")
        require_cap_equal(cap, state.root_cap, f"run {run_index} rejectFull")
        if status != ERR_EAGAIN:
            fail(f"run {run_index} rejectFull did not fail with EAGAIN")
        return

    if op == ops.object_create:
        if not can_root_mint(state):
            fail(f"run {run_index} objectCreate root mint precondition failed")
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
        require_cap_equal(cap, expected, f"run {run_index} objectCreate")
        if status != ERR_OK:
            fail(f"run {run_index} objectCreate failed")
        state.created_object_created = True
        state.minted_cap = cap
        if not cap_currently_authorizes(state, state.minted_cap):
            fail(f"run {run_index} objectCreate minted cap does not authorize created object")
        return

    if op == ops.cap_revoke:
        if not root_cap_currently_authorizes(state):
            fail(f"run {run_index} capRevoke root authority precondition failed")
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
        require_cap_equal(cap, expected, f"run {run_index} capRevoke")
        if status != ERR_OK:
            fail(f"run {run_index} capRevoke failed")
        state.object_gen = old_gen + 1
        state.revoked_gen = old_gen
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
            fail(f"run {run_index} rejectStale before consumer cap exists")
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
        require_cap_equal(cap, expected_live_projection, f"run {run_index} rejectStale")
        if status != ERR_EREVOKED:
            fail(f"run {run_index} rejectStale did not fail with EREVOKED")
        if cap_currently_authorizes(state, state.consumer_cap):
            fail(f"run {run_index} stale consumer cap still authorizes work")
        if state.revoked_gen is None or state.consumer_cap.fdr_gen != state.revoked_gen:
            fail(f"run {run_index} stale cap does not point at the revoked generation")
        state.stale_rejected = True
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


def check_run(run: list[dict[str, int | str]], index: int, ops: CommitOps) -> None:
    sequence = [require_int(record, "op") for record in run]
    if sequence == ops.denied_sequence:
        check_denied_run(run, index, ops)
        return
    if sequence != ops.expected_sequence:
        fail(f"run {index} op sequence {sequence} != {ops.expected_sequence} or {ops.denied_sequence}")

    for record in run:
        check_common(record, ops)

    state = initial_state(run[0])
    for record in run:
        apply_commit(state, record, index, ops)
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


def check_denied_run(run: list[dict[str, int | str]], index: int, ops: CommitOps) -> None:
    if len(run) != 1:
        fail(f"run {index} denied path emitted more than one commit")
    record = run[0]
    check_common(record, ops)
    cap = cap_from_record(record)
    expected = Cap(
        object_id=1,
        object_gen=1,
        fdr_gen=1,
        domain_id=1,
        domain_gen=1,
        rights_mask=RIGHT_PUSH | RIGHT_PULL,
        lineage_epoch=1,
        sealed=0,
    )
    require_cap_equal(cap, expected, f"run {index} capDupDenied")
    if require_int(record, "status") != ERR_EPERM:
        fail(f"run {index} capDupDenied did not report EPERM")
    if cap.rights_mask & RIGHT_DUP:
        fail(f"run {index} capDupDenied still carried dup authority")


def main() -> int:
    record_name, schema_fields, ops = load_schema_contract()
    output = run_m1_gate()
    records = parse_records(output, record_name, schema_fields)
    runs = split_runs(records, ops)
    for index, run in enumerate(runs):
        check_run(run, index, ops)
    print(f"rtl m1 typed commit trace ok ({len(runs)} run(s))")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

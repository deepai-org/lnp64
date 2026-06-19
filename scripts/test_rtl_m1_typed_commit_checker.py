#!/usr/bin/env python3
"""Self-test M1 typed commit checker Lean packed-schema failure modes."""

from __future__ import annotations

import copy
import contextlib
import importlib.util
import io
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m1_typed_commit_trace.py"
SCHEMA = ROOT / "rtl/schema/lnp64_shared_schema.json"
LEAN_M1_MODEL = ROOT / "formal/M1TransitionInvariantModel.lean"
RTL_M1_ENGINE = ROOT / "rtl/engines/lnp64_m1_pingpong.sv"
RTL_M1_TB = ROOT / "rtl/sim/lnp64_m1_tb.sv"
RTL_M1_ASSERTIONS = ROOT / "formal/rtl_assertions/lnp64_m1_assertions.sv"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m1_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M1 checker module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def schema_specs(checker) -> tuple[tuple[tuple[str, int], ...], tuple[tuple[str, int], ...]]:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    contract = schema["m1_typed_commit_contract"]
    commit_fields = tuple(
        checker.parse_schema_field(entry)
        for entry in schema["records"][contract["record"]]
    )
    state_fields = tuple(
        checker.parse_schema_field(entry)
        for entry in schema["records"][contract["state_record"]]
    )
    return commit_fields, state_fields


def expect_failure(expected: str, action) -> None:
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        try:
            action()
        except SystemExit as exc:
            require(exc.code != 0, "checker failure unexpectedly used success exit code")
        else:
            raise SystemExit("expected checker failure")
    output = stderr.getvalue()
    require(expected in output, f"checker failure did not include {expected!r}: {output}")


def replace_once(source: str, old: str, new: str) -> str:
    require(old in source, f"Lean source did not contain {old!r}")
    return source.replace(old, new, 1)


def m1_commit_record(
    op: int,
    object_id: int,
    object_gen: int,
    fdr_gen: int,
    domain_id: int,
    rights_mask: int,
    status: int,
) -> dict[str, int | str]:
    return {
        "record": "m1_cap_commit",
        "op": op,
        "object_id": object_id,
        "object_gen": object_gen,
        "fdr_gen": fdr_gen,
        "domain_id": domain_id,
        "domain_gen": 1,
        "rights_mask": rights_mask,
        "lineage_epoch": 1,
        "sealed": 0,
        "status": status,
    }


def build_valid_full_run(checker, ops) -> tuple[
    list[dict[str, int | str]],
    list[dict[str, int | str]],
]:
    run = [
        m1_commit_record(ops.cap_dup, 1, 1, 1, 2, checker.RIGHT_PULL, checker.ERR_OK),
        m1_commit_record(ops.cap_send, 1, 1, 1, 2, checker.RIGHT_PULL, checker.ERR_OK),
        m1_commit_record(ops.cap_recv, 1, 1, 1, 2, checker.RIGHT_PULL, checker.ERR_OK),
        m1_commit_record(ops.push, 1, 1, 1, 1, checker.ROOT_RIGHTS, checker.ERR_OK),
        m1_commit_record(ops.pull, 1, 1, 1, 2, checker.RIGHT_PULL, checker.ERR_OK),
        m1_commit_record(ops.reject_full, 1, 1, 1, 1, checker.ROOT_RIGHTS, checker.ERR_EAGAIN),
        m1_commit_record(ops.object_create, 2, 1, 1, 1, checker.ROOT_RIGHTS, checker.ERR_OK),
        m1_commit_record(ops.cap_revoke, 1, 2, 1, 1, checker.ROOT_RIGHTS, checker.ERR_OK),
        m1_commit_record(ops.reject_stale, 1, 2, 1, 2, checker.RIGHT_PULL, checker.ERR_EREVOKED),
    ]
    state = checker.initial_state(run[0], ops)
    state_run = []
    for record in run:
        checker.apply_commit(state, record, 0, ops)
        state_run.append(
            checker.projection_from_state(
                state,
                checker.require_int(record, "op"),
                checker.require_int(record, "status"),
            )
        )
    return run, state_run


def build_valid_denied_run(checker, ops) -> tuple[
    list[dict[str, int | str]],
    list[dict[str, int | str]],
]:
    run = [
        m1_commit_record(
            ops.cap_dup_denied,
            1,
            1,
            1,
            1,
            checker.RIGHT_PUSH | checker.RIGHT_PULL,
            checker.ERR_EPERM,
        )
    ]
    state = checker.initial_state(run[0], ops)
    checker.apply_commit(state, run[0], 0, ops)
    return run, [
        checker.projection_from_state(
            state,
            checker.require_int(run[0], "op"),
            checker.require_int(run[0], "status"),
        )
    ]


def main() -> None:
    checker = load_checker()
    commit_fields, state_fields = schema_specs(checker)
    commit_field_names = tuple(name for name, _width in commit_fields)
    state_field_names = tuple(name for name, _width in state_fields)
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    m1_contract = schema["m1_typed_commit_contract"]
    lean_source = LEAN_M1_MODEL.read_text(encoding="utf-8")
    engine_source = RTL_M1_ENGINE.read_text(encoding="utf-8")
    tb_source = RTL_M1_TB.read_text(encoding="utf-8")
    assertion_source = RTL_M1_ASSERTIONS.read_text(encoding="utf-8")

    checker.check_lean_packed_schema_contract(lean_source, commit_fields, state_fields)
    checker.check_rtl_state_projection_boundary_sources(
        engine_source,
        tb_source,
        assertion_source,
        commit_field_names,
        state_field_names,
    )

    checker.check_lean_typed_commit_mapping(
        checker.load_m1_op_mappings(m1_contract),
        checker.load_m1_status_mappings(m1_contract),
    )
    *_contract_prefix, ops = checker.load_schema_contract()

    valid_run, valid_state_run = build_valid_full_run(checker, ops)
    checker.check_run(valid_run, valid_state_run, 0, ops)

    valid_denied_run, valid_denied_state_run = build_valid_denied_run(checker, ops)
    checker.check_run(valid_denied_run, valid_denied_state_run, 0, ops)

    cap_send_before_dup_state = checker.initial_state(valid_run[0], ops)
    expect_failure(
        "TypedCommitTransition.capSend before capDup",
        lambda: checker.check_rtl_refinement_step(
            cap_send_before_dup_state,
            None,
            valid_run[1],
            valid_state_run[1],
            0,
            ops,
        ),
    )

    object_create_without_mint_state = checker.initial_state(valid_run[0], ops)
    object_create_without_mint_state.root_cap = checker.Cap(
        object_id=object_create_without_mint_state.root_cap.object_id,
        object_gen=object_create_without_mint_state.object_gen,
        fdr_gen=object_create_without_mint_state.object_gen,
        domain_id=object_create_without_mint_state.root_cap.domain_id,
        domain_gen=object_create_without_mint_state.root_cap.domain_gen,
        rights_mask=checker.RIGHT_PUSH | checker.RIGHT_PULL | checker.RIGHT_DUP,
        lineage_epoch=object_create_without_mint_state.root_cap.lineage_epoch,
        sealed=object_create_without_mint_state.root_cap.sealed,
    )
    expect_failure(
        "TypedCommitTransition.objectCreate root mint precondition failed",
        lambda: checker.check_rtl_refinement_step(
            object_create_without_mint_state,
            None,
            valid_run[6],
            valid_state_run[6],
            0,
            ops,
        ),
    )

    push_without_push_right_state = checker.initial_state(valid_run[0], ops)
    push_without_push_right_state.root_cap = checker.Cap(
        object_id=push_without_push_right_state.root_cap.object_id,
        object_gen=push_without_push_right_state.object_gen,
        fdr_gen=push_without_push_right_state.object_gen,
        domain_id=push_without_push_right_state.root_cap.domain_id,
        domain_gen=push_without_push_right_state.root_cap.domain_gen,
        rights_mask=checker.RIGHT_PULL | checker.RIGHT_DUP | checker.RIGHT_MINT,
        lineage_epoch=push_without_push_right_state.root_cap.lineage_epoch,
        sealed=push_without_push_right_state.root_cap.sealed,
    )
    expect_failure(
        "TypedCommitTransition.push root push precondition failed",
        lambda: checker.check_rtl_refinement_step(
            push_without_push_right_state,
            None,
            valid_run[3],
            valid_state_run[3],
            0,
            ops,
        ),
    )

    pull_empty_queue_state = checker.initial_state(valid_run[0], ops)
    for record in valid_run[:3]:
        checker.apply_commit(pull_empty_queue_state, record, 0, ops)
    expect_failure(
        "TypedCommitTransition.pull while queue is empty",
        lambda: checker.check_rtl_refinement_step(
            pull_empty_queue_state,
            valid_state_run[2],
            valid_run[4],
            valid_state_run[4],
            0,
            ops,
        ),
    )

    revoke_stale_root_state = checker.initial_state(valid_run[0], ops)
    revoke_stale_root_state.root_cap = checker.Cap(
        object_id=revoke_stale_root_state.root_cap.object_id,
        object_gen=revoke_stale_root_state.object_gen,
        fdr_gen=0,
        domain_id=revoke_stale_root_state.root_cap.domain_id,
        domain_gen=revoke_stale_root_state.root_cap.domain_gen,
        rights_mask=revoke_stale_root_state.root_cap.rights_mask,
        lineage_epoch=revoke_stale_root_state.root_cap.lineage_epoch,
        sealed=revoke_stale_root_state.root_cap.sealed,
    )
    expect_failure(
        "TypedCommitTransition.capRevoke root authority precondition failed",
        lambda: checker.check_rtl_refinement_step(
            revoke_stale_root_state,
            None,
            valid_run[7],
            valid_state_run[7],
            0,
            ops,
        ),
    )

    reject_stale_live_cap_state = checker.initial_state(valid_run[0], ops)
    checker.apply_commit(reject_stale_live_cap_state, valid_run[0], 0, ops)
    live_reject_stale = m1_commit_record(
        ops.reject_stale,
        1,
        1,
        1,
        2,
        checker.RIGHT_PULL,
        checker.ERR_EREVOKED,
    )
    expect_failure(
        "TypedCommitTransition.rejectStale stale consumer cap still authorizes work",
        lambda: checker.check_rtl_refinement_step(
            reject_stale_live_cap_state,
            valid_state_run[0],
            live_reject_stale,
            valid_state_run[8],
            0,
            ops,
        ),
    )

    bad_post_state_run = copy.deepcopy(valid_state_run)
    bad_post_state_run[1]["transfer_valid"] = 0
    expect_failure(
        "RtlM1RefinementStep post-state projection",
        lambda: checker.check_run(valid_run, bad_post_state_run, 0, ops),
    )

    bad_cap_dup_postcondition_run = copy.deepcopy(valid_state_run)
    bad_cap_dup_postcondition_run[0]["consumer_rights"] = checker.ROOT_RIGHTS
    expect_failure(
        "post consumer projection field consumer_rights",
        lambda: checker.check_run(valid_run, bad_cap_dup_postcondition_run, 0, ops),
    )

    bad_cap_send_postcondition_run = copy.deepcopy(valid_state_run)
    bad_cap_send_postcondition_run[1]["sent_valid"] = 0
    expect_failure(
        "capSend postcondition did not publish sent cap",
        lambda: checker.check_run(valid_run, bad_cap_send_postcondition_run, 0, ops),
    )

    bad_cap_recv_postcondition_run = copy.deepcopy(valid_state_run)
    bad_cap_recv_postcondition_run[2]["sent_valid"] = 1
    expect_failure(
        "capRecv postcondition did not clear sent cap",
        lambda: checker.check_run(valid_run, bad_cap_recv_postcondition_run, 0, ops),
    )

    bad_non_ok_authority_state_run = copy.deepcopy(valid_state_run)
    bad_non_ok_authority_state_run[5]["root_rights"] = checker.ROOT_RIGHTS ^ checker.RIGHT_MINT
    expect_failure(
        "non-OK commit changed authority projection field root_rights",
        lambda: checker.check_run(valid_run, bad_non_ok_authority_state_run, 0, ops),
    )

    bad_push_postcondition_run = copy.deepcopy(valid_state_run)
    bad_push_postcondition_run[3]["wake_pending"] = 0
    expect_failure(
        "push postcondition did not set wake_pending",
        lambda: checker.check_run(valid_run, bad_push_postcondition_run, 0, ops),
    )

    bad_pull_postcondition_run = copy.deepcopy(valid_state_run)
    bad_pull_postcondition_run[4]["wake_pending"] = 1
    expect_failure(
        "pull postcondition did not clear wake_pending",
        lambda: checker.check_run(valid_run, bad_pull_postcondition_run, 0, ops),
    )

    bad_reject_full_postcondition_run = copy.deepcopy(valid_state_run)
    bad_reject_full_postcondition_run[5]["full_was_explicit"] = 0
    expect_failure(
        "rejectFull postcondition did not set full_was_explicit",
        lambda: checker.check_run(valid_run, bad_reject_full_postcondition_run, 0, ops),
    )

    bad_reject_stale_postcondition_run = copy.deepcopy(valid_state_run)
    bad_reject_stale_postcondition_run[8]["stale_rejected"] = 0
    expect_failure(
        "rejectStale postcondition did not set stale_rejected",
        lambda: checker.check_run(valid_run, bad_reject_stale_postcondition_run, 0, ops),
    )

    bad_object_create_postcondition_run = copy.deepcopy(valid_state_run)
    bad_object_create_postcondition_run[6]["minted_valid"] = 0
    expect_failure(
        "objectCreate postcondition did not mint a cap",
        lambda: checker.check_run(valid_run, bad_object_create_postcondition_run, 0, ops),
    )

    bad_cap_revoke_postcondition_run = copy.deepcopy(valid_state_run)
    bad_cap_revoke_postcondition_run[7]["has_revoked_generation"] = 0
    expect_failure(
        "capRevoke postcondition did not publish revoked-generation witness",
        lambda: checker.check_run(valid_run, bad_cap_revoke_postcondition_run, 0, ops),
    )

    bad_cap_dup_denied_postcondition_run = copy.deepcopy(valid_denied_state_run)
    bad_cap_dup_denied_postcondition_run[0]["failed_no_authority"] = 0
    expect_failure(
        "capDupDenied postcondition did not set failed_no_authority",
        lambda: checker.check_run(valid_denied_run, bad_cap_dup_denied_postcondition_run, 0, ops),
    )

    pre_state = checker.initial_state(valid_run[0], ops)
    checker.apply_commit(pre_state, valid_run[0], 0, ops)
    bad_pre_state_record = copy.deepcopy(valid_state_run[0])
    bad_pre_state_record["consumer_rights"] = checker.ROOT_RIGHTS
    expect_failure(
        "RtlM1RefinementStep pre-state projection",
        lambda: checker.check_rtl_refinement_step(
            pre_state,
            bad_pre_state_record,
            valid_run[1],
            valid_state_run[1],
            0,
            ops,
        ),
    )

    bad_op_key_contract = json.loads(json.dumps(m1_contract))
    bad_op_key_contract["op_mappings"][0]["key"] = "cap_dup_renamed"
    expect_failure(
        "op mapping keys drifted",
        lambda: checker.load_m1_op_mappings(bad_op_key_contract),
    )

    bad_lean_transition_contract = json.loads(json.dumps(m1_contract))
    bad_lean_transition_contract["op_mappings"][0]["lean_transition"] = "missingLeanTransition"
    expect_failure(
        "missing Lean TypedCommitTransition constructors",
        lambda: checker.check_lean_typed_commit_mapping(
            checker.load_m1_op_mappings(bad_lean_transition_contract),
            checker.load_m1_status_mappings(bad_lean_transition_contract),
        ),
    )

    missing_refinement_postcondition_theorem = replace_once(
        lean_source,
        "theorem rtl_m1_refinement_step_satisfies_postcondition",
        "theorem rtl_m1_refinement_step_satisfies_postcondition_missing",
    )
    original_load_lean_model_source = checker.load_lean_model_source
    checker.load_lean_model_source = lambda: missing_refinement_postcondition_theorem
    try:
        expect_failure(
            "typed commit bridge theorems are missing from Lean",
            lambda: checker.check_lean_typed_commit_mapping(
                checker.load_m1_op_mappings(m1_contract),
                checker.load_m1_status_mappings(m1_contract),
            ),
        )
    finally:
        checker.load_lean_model_source = original_load_lean_model_source

    expect_failure(
        "every commit trace field from typed_commit",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            tb_source,
            assertion_source,
            commit_field_names + ("schema_added_commit_field",),
            state_field_names,
        ),
    )

    expect_failure(
        "every state trace field from typed_state_projection",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            tb_source,
            assertion_source,
            commit_field_names,
            state_field_names + ("schema_added_state_field",),
        ),
    )

    missing_commit_field = replace_once(
        tb_source,
        "typed_commit.object_id,",
        "typed_commit.object_gen,",
    )
    expect_failure(
        "every commit trace field from typed_commit",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            missing_commit_field,
            assertion_source,
            commit_field_names,
            state_field_names,
        ),
    )

    wrong_commit_bits_source = replace_once(
        tb_source,
        "typed_commit\n            );\n            $display(\n                \"TTRACE_M1_STATE",
        "typed_state_projection\n            );\n            $display(\n                \"TTRACE_M1_STATE",
    )
    expect_failure(
        "packed commit bits from typed_commit",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            wrong_commit_bits_source,
            assertion_source,
            commit_field_names,
            state_field_names,
        ),
    )

    projection_derived_queue_generation = replace_once(
        tb_source,
        ".queue_generation(dut.queue_generation)",
        ".queue_generation(typed_state_projection.object_gen)",
    )
    expect_failure(
        "real RTL authority state into projection faithfulness assertions",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            projection_derived_queue_generation,
            assertion_source,
            commit_field_names,
            state_field_names,
        ),
    )

    missing_sent_cap_state_connection = replace_once(
        tb_source,
        ".sent_cap_state(dut.sent_cap_state)",
        ".sent_cap_state(typed_state_projection.sent_object_id)",
    )
    expect_failure(
        "real RTL authority state into projection faithfulness assertions",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            missing_sent_cap_state_connection,
            assertion_source,
            commit_field_names,
            state_field_names,
        ),
    )

    projection_derived_from_ambient_state = replace_once(
        engine_source,
        "typed_state_projection.sent_generation = sent_cap_state.fdr_gen",
        "typed_state_projection.sent_generation = consumer_fd_generation",
    )
    expect_failure(
        "explicit RTL cap-state slots",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            projection_derived_from_ambient_state,
            tb_source,
            assertion_source,
            commit_field_names,
            state_field_names,
        ),
    )

    missing_transfer_mediation_assertion = replace_once(
        assertion_source,
        "M1 sent-cap validity set outside capSend owner path",
        "M1 sent-cap validity set outside unchecked path",
    )
    expect_failure(
        "mediate transfer/mint validity transitions",
        lambda: checker.check_rtl_state_projection_boundary_sources(
            engine_source,
            tb_source,
            missing_transfer_mediation_assertion,
            commit_field_names,
            state_field_names,
        ),
    )

    missing_commit_schema = replace_once(
        lean_source,
        "def rtlM1CommitPackedSchema :",
        "def rtlM1CommitPackedSchemaMissing :",
    )
    expect_failure(
        "missing packed schema rtlM1CommitPackedSchema",
        lambda: checker.check_lean_packed_schema_contract(
            missing_commit_schema,
            commit_fields,
            state_fields,
        ),
    )

    wrong_commit_width = replace_once(
        lean_source,
        "packedSchemaWidth rtlM1CommitPackedSchema = 281",
        "packedSchemaWidth rtlM1CommitPackedSchema = 280",
    )
    expect_failure(
        "rtlM1CommitPackedSchema_width drifted",
        lambda: checker.check_lean_packed_schema_contract(
            wrong_commit_width,
            commit_fields,
            state_fields,
        ),
    )

    wrong_state_width = replace_once(
        lean_source,
        "packedSchemaWidth rtlM1StateProjectionPackedSchema = 902",
        "packedSchemaWidth rtlM1StateProjectionPackedSchema = 901",
    )
    expect_failure(
        "rtlM1StateProjectionPackedSchema_width drifted",
        lambda: checker.check_lean_packed_schema_contract(
            wrong_state_width,
            commit_fields,
            state_fields,
        ),
    )

    wrong_field_width = replace_once(
        lean_source,
        '("rights_mask", 64)',
        '("rights_mask", 63)',
    )
    expect_failure(
        "rtlM1CommitPackedSchema drifted",
        lambda: checker.check_lean_packed_schema_contract(
            wrong_field_width,
            commit_fields,
            state_fields,
        ),
    )

    print("rtl m1 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Self-test M7 typed scheduler/wakeup commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
import tempfile
from pathlib import Path


sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m7_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m7_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M7 checker module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def expect_failure(expected: str, action) -> None:
    try:
        action()
    except SystemExit as exc:
        require(exc.code != 0, "checker failure unexpectedly used success exit code")
        output = str(exc)
        require(expected in output, f"checker failure did not include {expected!r}: {output}")
    else:
        raise SystemExit("expected checker failure")


def replace_once(text: str, old: str, new: str) -> str:
    require(old in text, f"missing text to replace: {old!r}")
    return text.replace(old, new, 1)


def commit_record(
    checker,
    op: int,
    status: int,
    before_location: int,
    after_location: int,
) -> dict[str, int | str]:
    return {
        "record": checker.COMMIT_NAME,
        "op": op,
        "status": status,
        "tid": checker.M7_TID,
        "before_location": before_location,
        "after_location": after_location,
        "wait_generation": 1,
        "address_generation": 1,
    }


def build_valid_run(checker, ops) -> tuple[
    list[dict[str, int | str]],
    list[dict[str, int | str]],
]:
    commits = [
        commit_record(checker, ops.cmpxchg_success, checker.ERR_OK, checker.LOC_RUNNABLE, checker.LOC_RUNNABLE),
        commit_record(checker, ops.cmpxchg_fail, checker.ERR_EAGAIN, checker.LOC_RUNNABLE, checker.LOC_RUNNABLE),
        commit_record(checker, ops.futex_wait, checker.ERR_OK, checker.LOC_RUNNABLE, checker.LOC_PARKED),
        commit_record(checker, ops.futex_wake, checker.ERR_OK, checker.LOC_PARKED, checker.LOC_RUNNABLE),
        commit_record(checker, ops.consume_wake, checker.ERR_OK, checker.LOC_RUNNABLE, checker.LOC_RUNNABLE),
        commit_record(checker, ops.timer_wait, checker.ERR_OK, checker.LOC_RUNNABLE, checker.LOC_PARKED),
        commit_record(checker, ops.timer_expire, checker.ERR_OK, checker.LOC_PARKED, checker.LOC_RUNNABLE),
        commit_record(checker, ops.reject_stale_address, checker.ERR_EREVOKED, checker.LOC_RUNNABLE, checker.LOC_RUNNABLE),
    ]
    state = checker.State()
    states: list[dict[str, int | str]] = []
    for commit in commits:
        op = checker.require_int(commit, "op")
        if op == ops.cmpxchg_success:
            state.atomic_word = 1
            state.atomic_count = 1
        elif op == ops.cmpxchg_fail:
            state.atomic_count = 2
            state.cmpxchg_failure_explicit = True
        elif op == ops.futex_wait:
            state.location = checker.LOC_PARKED
            state.wait_generation = state.address_generation
        elif op == ops.futex_wake:
            state.location = checker.LOC_RUNNABLE
            state.wake_pending = True
            state.futex_wake_delivered = True
        elif op == ops.consume_wake:
            state.wake_pending = False
        elif op == ops.timer_wait:
            state.location = checker.LOC_PARKED
            state.wait_generation = state.address_generation
        elif op == ops.timer_expire:
            state.location = checker.LOC_RUNNABLE
            state.wake_pending = True
            state.timer_wake_delivered = True
        elif op == ops.reject_stale_address:
            state.stale_address_rejected = True
        else:
            raise AssertionError(f"unexpected op {op}")
        projection = state.as_projection(op, checker.require_int(commit, "status"))
        projection["record"] = checker.STATE_NAME
        states.append(projection)
    return commits, states


def encode_bits(record: dict[str, int | str], fields: tuple[str, ...], widths: tuple[int, ...]) -> str:
    value = 0
    for field, width in zip(fields, widths, strict=True):
        raw = record[field]
        require(isinstance(raw, int), f"{field} must be an integer")
        require(0 <= raw < (1 << width), f"{field}={raw} does not fit in {width} bits")
        value = (value << width) | raw
    hex_digits = (sum(widths) + 3) // 4
    return f"{value:0{hex_digits}x}"


def main() -> None:
    checker = load_checker()
    commit_fields, commit_widths, state_fields, state_widths, ops = checker.load_schema()
    commits, states = build_valid_run(checker, ops)

    checker.check_transition_trace(commits, states, ops)

    missing_consume = commits[:4] + commits[5:]
    missing_consume_states = states[:4] + states[5:]
    expect_failure(
        "M7 typed commit sequence drifted",
        lambda: checker.check_transition_trace(missing_consume, missing_consume_states, ops),
    )

    wrong_projection = copy.deepcopy(states)
    wrong_projection[3]["wake_pending"] = 0
    expect_failure(
        "state projection 3 field wake_pending drifted",
        lambda: checker.check_transition_trace(commits, wrong_projection, ops),
    )

    wrong_status = copy.deepcopy(commits)
    wrong_status[1]["status"] = checker.ERR_OK
    expect_failure(
        "unexpected status",
        lambda: checker.check_transition_trace(wrong_status, states, ops),
    )

    wrong_before_location = copy.deepcopy(commits)
    wrong_before_location[3]["before_location"] = checker.LOC_RUNNABLE
    expect_failure(
        "unexpected before location",
        lambda: checker.check_transition_trace(wrong_before_location, states, ops),
    )

    commit_bits = [encode_bits(record, commit_fields, commit_widths) for record in commits]
    state_bits = [encode_bits(record, state_fields, state_widths) for record in states]
    checker.check_bits(commits, commit_bits, commit_fields, commit_widths, "M7 typed commit")
    checker.check_bits(states, state_bits, state_fields, state_widths, "M7 state projection")

    valid_commit_bit_record_output = (
        "TTRACE_M7_BITS "
        f'{{"record":"m7_sched_commit_bits","width":{sum(commit_widths)},'
        f'"bits":"{commit_bits[0]}"}}\n'
    )
    checker.parse_bit_records(
        valid_commit_bit_record_output,
        "TTRACE_M7_BITS ",
        checker.COMMIT_BITS_NAME,
        sum(commit_widths),
    )

    missing_width_commit_bit_record_output = (
        "TTRACE_M7_BITS "
        f'{{"record":"m7_sched_commit_bits","bits":"{commit_bits[0]}"}}\n'
    )
    expect_failure(
        "packed bit record m7_sched_commit_bits width drifted from schema",
        lambda: checker.parse_bit_records(
            missing_width_commit_bit_record_output,
            "TTRACE_M7_BITS ",
            checker.COMMIT_BITS_NAME,
            sum(commit_widths),
        ),
    )

    wrong_width_state_bit_record_output = (
        "TTRACE_M7_STATE_BITS "
        f'{{"record":"m7_state_projection_bits","width":{sum(state_widths) - 1},'
        f'"bits":"{state_bits[0]}"}}\n'
    )
    expect_failure(
        "packed bit record m7_state_projection_bits width drifted from schema",
        lambda: checker.parse_bit_records(
            wrong_width_state_bit_record_output,
            "TTRACE_M7_STATE_BITS ",
            checker.STATE_BITS_NAME,
            sum(state_widths),
        ),
    )

    tb_source = checker.RTL_M7_TB.read_text(encoding="utf-8")
    checker.check_m7_testbench_trace_source_contract()
    original_tb_path = checker.RTL_M7_TB
    try:
        with tempfile.TemporaryDirectory(prefix="lnp64-m7-source-contract-") as raw_tmp:
            tmp = Path(raw_tmp)
            missing_width_tb = tmp / "missing_width.sv"
            missing_width_tb.write_text(
                replace_once(tb_source, "$bits(lnp64_m7_sched_commit_t)", "152"),
                encoding="utf-8",
            )
            checker.RTL_M7_TB = missing_width_tb
            expect_failure(
                "M7 testbench no longer emits schema-owned packed bit widths",
                checker.check_m7_testbench_trace_source_contract,
            )

            wrong_payload_tb = tmp / "wrong_payload.sv"
            wrong_payload_tb.write_text(
                replace_once(
                    tb_source,
                    "                typed_commit\n            );",
                    "                typed_state_projection\n            );",
                ),
                encoding="utf-8",
            )
            checker.RTL_M7_TB = wrong_payload_tb
            expect_failure(
                "M7 testbench no longer emits packed commit bits from typed_commit",
                checker.check_m7_testbench_trace_source_contract,
            )
    finally:
        checker.RTL_M7_TB = original_tb_path

    bad_commit_bits = list(commit_bits)
    bad_commit_bits[0] = encode_bits({**commits[0], "tid": checker.M7_TID + 1}, commit_fields, commit_widths)
    expect_failure(
        "M7 typed commit packed decode drift",
        lambda: checker.check_bits(commits, bad_commit_bits, commit_fields, commit_widths, "M7 typed commit"),
    )

    bad_state_bits = list(state_bits)
    bad_state_bits[0] = encode_bits({**states[0], "atomic_count": 0}, state_fields, state_widths)
    expect_failure(
        "M7 state projection packed decode drift",
        lambda: checker.check_bits(states, bad_state_bits, state_fields, state_widths, "M7 state projection"),
    )

    print("rtl m7 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

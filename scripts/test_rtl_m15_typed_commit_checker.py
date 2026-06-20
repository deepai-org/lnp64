#!/usr/bin/env python3
"""Self-test the M15 typed object-profiles commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m15_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m15_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M15 checker module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def expect_failure(expected: str, action) -> None:
    try:
        action()
    except SystemExit as exc:
        require(exc.code != 0, "checker failure unexpectedly used success exit code")
        require(expected in str(exc), f"checker failure did not include {expected!r}: {exc}")
    else:
        raise SystemExit(f"expected checker failure for: {expected}")


def encode_bits(record: dict, fields: tuple[str, ...], widths: tuple[int, ...]) -> str:
    value = 0
    for field, width in zip(fields, widths, strict=True):
        raw = record[field]
        require(isinstance(raw, int) and 0 <= raw < (1 << width), f"{field}={raw} does not fit in {width} bits")
        value = (value << width) | raw
    return f"{value:0{(sum(widths) + 3) // 4}x}"


def build_valid(checker):
    cf, cw, sf, sw, ops = checker.load_schema()

    def commit(op, status, threshold=3):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "object_id": 1,
                "generation": 1, "threshold": threshold, "payload": 42, "event_generation": 1,
                "continuation": 1}

    def state(op, status, failures, events, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "failures": failures, "events": events})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M15_COMMIT_COUNTER"], checker.ERR_OK),
        commit(o["LNP64_M15_COMMIT_QUEUE_PUSH"], checker.ERR_OK),
        commit(o["LNP64_M15_COMMIT_QUEUE_OVERFLOW"], checker.ERR_EAGAIN),
        commit(o["LNP64_M15_COMMIT_EVENT_EMIT"], checker.ERR_OK),
        commit(o["LNP64_M15_COMMIT_STALE_EVENT"], checker.ERR_EREVOKED),
        commit(o["LNP64_M15_COMMIT_GATE_PROFILE"], checker.ERR_EREVOKED),
    ]
    done = dict(counter_threshold_event=1, queue_rights_valid=1, queue_overflow_explicit=1,
                event_source_generation_safe=1)
    states = [
        state(o["LNP64_M15_COMMIT_COUNTER"], checker.ERR_OK, 0, 1, counter_threshold_event=1),
        state(o["LNP64_M15_COMMIT_QUEUE_PUSH"], checker.ERR_OK, 0, 1, counter_threshold_event=1, queue_rights_valid=1),
        state(o["LNP64_M15_COMMIT_QUEUE_OVERFLOW"], checker.ERR_EAGAIN, 1, 2, counter_threshold_event=1, queue_rights_valid=1, queue_overflow_explicit=1),
        state(o["LNP64_M15_COMMIT_EVENT_EMIT"], checker.ERR_OK, 1, 2, counter_threshold_event=1, queue_rights_valid=1, queue_overflow_explicit=1),
        state(o["LNP64_M15_COMMIT_STALE_EVENT"], checker.ERR_EREVOKED, 2, 2, counter_threshold_event=1, queue_rights_valid=1, queue_overflow_explicit=1, event_source_generation_safe=1),
        state(o["LNP64_M15_COMMIT_GATE_PROFILE"], checker.ERR_EREVOKED, 3, 2, **done, gate_continuation_unique=1, counts_exact=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M15 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M15 state projection")
    check_transition = checker.check_transition_trace
    check_transition(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: check_transition(scrambled, states, ops))

    # The counter must raise the threshold event.
    bad_counter = copy.deepcopy(states)
    bad_counter[0]["counter_threshold_event"] = 0
    expect_failure("did not raise the threshold event", lambda: check_transition(commits, bad_counter, ops))

    # Queue overflow must be an explicit EAGAIN, never a silent drop.
    bad_overflow = copy.deepcopy(states)
    bad_overflow[2]["queue_overflow_explicit"] = 0
    expect_failure("was not an explicit EAGAIN", lambda: check_transition(commits, bad_overflow, ops))

    # A stale event source must be rejected as revoked.
    bad_stale = copy.deepcopy(states)
    bad_stale[4]["event_source_generation_safe"] = 0
    expect_failure("did not reject the stale source generation", lambda: check_transition(commits, bad_stale, ops))

    # The terminal gate profile must see the prior object invariants established.
    bad_order = copy.deepcopy(states)
    bad_order[5]["event_source_generation_safe"] = 0
    expect_failure("preceded the object invariants", lambda: check_transition(commits, bad_order, ops))

    # An unbound object/generation must be rejected.
    bad_commit = copy.deepcopy(commits)
    bad_commit[0]["generation"] = 0
    expect_failure("unbound object/generation", lambda: check_transition(bad_commit, states, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "threshold": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M15 typed commit"))

    print("rtl m15 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

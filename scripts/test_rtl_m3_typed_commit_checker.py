#!/usr/bin/env python3
"""Self-test the M3 typed process/thread-lifecycle commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m3_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m3_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M3 checker module")
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

    def commit(op, status, ctid, cgen, jgen, epoch, exit_code):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "parent_tid": 1,
                "child_tid": ctid, "child_generation": cgen, "join_generation": jgen,
                "exec_epoch": epoch, "exit_code": exit_code}

    def state(op, status, ctid, cgen, jgen, epoch, child_state, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "parent_tid": 1, "child_tid": ctid,
                     "child_generation": cgen, "join_generation": jgen, "exec_epoch": epoch,
                     "parent_state": 2, "child_state": child_state, "exactly_one_thread_location": 1})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M3_COMMIT_CLONE"], checker.ERR_OK, 0, 0, 0, 1, 0),
        commit(o["LNP64_M3_COMMIT_CHILD_EXIT"], checker.ERR_OK, 2, 1, 1, 1, 7),
        commit(o["LNP64_M3_COMMIT_PARENT_JOIN"], checker.ERR_OK, 2, 1, 1, 1, 7),
        commit(o["LNP64_M3_COMMIT_EXEC_BARRIER"], checker.ERR_OK, 2, 2, 1, 1, 0),
        commit(o["LNP64_M3_COMMIT_STALE_JOIN"], checker.ERR_EREVOKED, 2, 2, 1, 2, 0),
        commit(o["LNP64_M3_COMMIT_EXEC_CANCEL"], checker.ERR_ECANCELED, 2, 2, 1, 2, 0),
    ]
    states = [
        state(o["LNP64_M3_COMMIT_CLONE"], checker.ERR_OK, 2, 1, 1, 1, 1, clone_created=1),
        state(o["LNP64_M3_COMMIT_CHILD_EXIT"], checker.ERR_OK, 2, 1, 1, 1, 3, clone_created=1, child_exit_signaled=1),
        state(o["LNP64_M3_COMMIT_PARENT_JOIN"], checker.ERR_OK, 2, 2, 1, 1, 0, clone_created=1, child_exit_signaled=1, parent_join_completed=1),
        state(o["LNP64_M3_COMMIT_EXEC_BARRIER"], checker.ERR_OK, 2, 2, 1, 1, 0, clone_created=1, child_exit_signaled=1, parent_join_completed=1, exec_barrier_stopped_sibling=1),
        state(o["LNP64_M3_COMMIT_STALE_JOIN"], checker.ERR_EREVOKED, 2, 2, 1, 2, 0, clone_created=1, child_exit_signaled=1, parent_join_completed=1, exec_barrier_stopped_sibling=1, stale_join_rejected=1),
        state(o["LNP64_M3_COMMIT_EXEC_CANCEL"], checker.ERR_ECANCELED, 2, 2, 1, 2, 0, clone_created=1, child_exit_signaled=1, parent_join_completed=1, exec_barrier_stopped_sibling=1, stale_join_rejected=1, exec_cancel_terminal=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M3 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M3 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # Scheduler uniqueness violated.
    bad_one = copy.deepcopy(states)
    bad_one[0]["exactly_one_thread_location"] = 0
    expect_failure("exactly-one-thread-location", lambda: checker.check_transition_trace(commits, bad_one, ops))

    # Stale join not rejected.
    bad_stale = copy.deepcopy(states)
    bad_stale[4]["stale_join_rejected"] = 0
    expect_failure("did not reject the stale join", lambda: checker.check_transition_trace(commits, bad_stale, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "parent_tid": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M3 typed commit"))

    print("rtl m3 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

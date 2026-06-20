#!/usr/bin/env python3
"""Self-test the M14 typed Resource Domain commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m14_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m14_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M14 checker module")
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
    REQ, DELEG = 7, 3

    def commit(op, status, cbud, pbud):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "root_domain": 1,
                "child_domain": 2, "child_budget": cbud, "parent_budget": pbud,
                "requested_rights": REQ, "delegated_rights": DELEG}

    def state(op, status, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "root_domain": 1, "child_domain": 2})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M14_COMMIT_DELEGATE"], checker.ERR_OK, 0, 0),
        commit(o["LNP64_M14_COMMIT_CREATE_CHILD"], checker.ERR_OK, 40, 100),
        commit(o["LNP64_M14_COMMIT_EXCESS_BUDGET"], checker.ERR_EPERM, 101, 100),
        commit(o["LNP64_M14_COMMIT_FREEZE"], checker.ERR_EAGAIN, 0, 0),
        commit(o["LNP64_M14_COMMIT_RESUME"], checker.ERR_OK, 0, 0),
        commit(o["LNP64_M14_COMMIT_USAGE"], checker.ERR_OK, 0, 0),
        commit(o["LNP64_M14_COMMIT_DESTROY"], checker.ERR_EREVOKED, 0, 0),
        commit(o["LNP64_M14_COMMIT_POLICY"], checker.ERR_OK, 0, 0),
    ]
    states = [
        state(o["LNP64_M14_COMMIT_DELEGATE"], checker.ERR_OK, child_rights_subset_parent=1),
        state(o["LNP64_M14_COMMIT_CREATE_CHILD"], checker.ERR_OK, child_rights_subset_parent=1, child_budget_within_parent=1),
        state(o["LNP64_M14_COMMIT_EXCESS_BUDGET"], checker.ERR_EPERM, child_rights_subset_parent=1, child_budget_within_parent=1, excess_budget_rejected=1, failures=1),
        state(o["LNP64_M14_COMMIT_FREEZE"], checker.ERR_EAGAIN, child_rights_subset_parent=1, child_budget_within_parent=1, excess_budget_rejected=1, frozen_dispatch_rejected=1, failures=2),
        state(o["LNP64_M14_COMMIT_RESUME"], checker.ERR_OK, child_rights_subset_parent=1, child_budget_within_parent=1, excess_budget_rejected=1, frozen_dispatch_rejected=1, resumed_dispatch_allowed=1, failures=2),
        state(o["LNP64_M14_COMMIT_USAGE"], checker.ERR_OK, child_rights_subset_parent=1, child_budget_within_parent=1, excess_budget_rejected=1, frozen_dispatch_rejected=1, resumed_dispatch_allowed=1, usage_rollup_valid=1, parent_used=20, failures=2),
        state(o["LNP64_M14_COMMIT_DESTROY"], checker.ERR_EREVOKED, child_rights_subset_parent=1, child_budget_within_parent=1, excess_budget_rejected=1, frozen_dispatch_rejected=1, resumed_dispatch_allowed=1, usage_rollup_valid=1, destroyed_dispatch_rejected=1, parent_used=20, failures=3),
        state(o["LNP64_M14_COMMIT_POLICY"], checker.ERR_OK, child_rights_subset_parent=1, child_budget_within_parent=1, excess_budget_rejected=1, frozen_dispatch_rejected=1, resumed_dispatch_allowed=1, usage_rollup_valid=1, destroyed_dispatch_rejected=1, policy_fail_closed=1, parent_used=20, failures=3),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M14 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M14 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # Monotonic delegation: delegated rights cannot exceed requested.
    amplify = copy.deepcopy(commits)
    amplify[0]["delegated_rights"] = 0xF
    expect_failure("amplified delegated rights", lambda: checker.check_transition_trace(amplify, states, ops))

    # Excess budget that was not actually over the parent.
    bad_excess = copy.deepcopy(commits)
    bad_excess[2]["child_budget"] = 50
    expect_failure("not actually over the parent budget", lambda: checker.check_transition_trace(bad_excess, states, ops))

    # Delegate without confining child rights.
    bad_deleg = copy.deepcopy(states)
    bad_deleg[0]["child_rights_subset_parent"] = 0
    expect_failure("did not confine child rights", lambda: checker.check_transition_trace(commits, bad_deleg, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "child_domain": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M14 typed commit"))

    print("rtl m14 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

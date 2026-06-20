#!/usr/bin/env python3
"""Self-test the M2 typed gate/continuation commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m2_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m2_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M2 checker module")
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
    MODE_SYNC, MODE_ASYNC, MODE_HANDOFF = 0, 1, 2

    def commit(op, status, cid, gen, mode):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "continuation_id": cid,
                "continuation_generation": gen, "caller_tid": 1, "callee_tid": 2, "mode": mode}

    def state(op, status, cid, gen, cont_valid, caller_loc, callee_loc, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "continuation_id": cid, "continuation_generation": gen,
                     "continuation_valid": cont_valid, "caller_loc": caller_loc, "callee_loc": callee_loc,
                     "continuation_unique": 1})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M2_COMMIT_SYNC_CALL"], checker.ERR_OK, 0, 1, MODE_SYNC),
        commit(o["LNP64_M2_COMMIT_SYNC_RETURN"], checker.ERR_OK, 1, 2, MODE_SYNC),
        commit(o["LNP64_M2_COMMIT_ASYNC_CALL"], checker.ERR_OK, 1, 2, MODE_ASYNC),
        commit(o["LNP64_M2_COMMIT_HANDOFF_CALL"], checker.ERR_OK, 1, 2, MODE_HANDOFF),
        commit(o["LNP64_M2_COMMIT_STALE_RETURN"], checker.ERR_EREVOKED, 1, 2, MODE_SYNC),
        commit(o["LNP64_M2_COMMIT_FAULT_DELIVERY"], checker.ERR_EFAULT, 1, 2, MODE_SYNC),
        commit(o["LNP64_M2_COMMIT_SIGNAL_COMPAT"], checker.ERR_OK, 1, 2, MODE_SYNC),
    ]
    states = [
        state(o["LNP64_M2_COMMIT_SYNC_CALL"], checker.ERR_OK, 1, 1, 1, 2, 1, continuation_valid=1),
        state(o["LNP64_M2_COMMIT_SYNC_RETURN"], checker.ERR_OK, 1, 2, 0, 0, 0, sync_roundtrip_ok=1),
        state(o["LNP64_M2_COMMIT_ASYNC_CALL"], checker.ERR_OK, 1, 2, 0, 0, 0, sync_roundtrip_ok=1, async_delivery_ok=1),
        state(o["LNP64_M2_COMMIT_HANDOFF_CALL"], checker.ERR_OK, 1, 2, 0, 0, 1, sync_roundtrip_ok=1, async_delivery_ok=1, handoff_delivery_ok=1),
        state(o["LNP64_M2_COMMIT_STALE_RETURN"], checker.ERR_EREVOKED, 1, 2, 0, 0, 0, sync_roundtrip_ok=1, async_delivery_ok=1, handoff_delivery_ok=1, stale_continuation_rejected=1),
        state(o["LNP64_M2_COMMIT_FAULT_DELIVERY"], checker.ERR_EFAULT, 1, 2, 0, 0, 0, delivered_faults=1, sync_roundtrip_ok=1, async_delivery_ok=1, handoff_delivery_ok=1, stale_continuation_rejected=1, fault_delivery_gate_ok=1),
        state(o["LNP64_M2_COMMIT_SIGNAL_COMPAT"], checker.ERR_OK, 1, 2, 0, 0, 0, delivered_faults=1, sync_roundtrip_ok=1, async_delivery_ok=1, handoff_delivery_ok=1, stale_continuation_rejected=1, fault_delivery_gate_ok=1, signal_compatibility_ok=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M2 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M2 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # Continuation uniqueness violated.
    bad_unique = copy.deepcopy(states)
    bad_unique[0]["continuation_unique"] = 0
    expect_failure("continuation uniqueness", lambda: checker.check_transition_trace(commits, bad_unique, ops))

    # Sync return that leaves a live continuation.
    bad_return = copy.deepcopy(states)
    bad_return[1]["continuation_valid"] = 1
    expect_failure("left a live continuation", lambda: checker.check_transition_trace(commits, bad_return, ops))

    # Stale return that does not reject.
    bad_stale = copy.deepcopy(states)
    bad_stale[4]["stale_continuation_rejected"] = 0
    expect_failure("did not reject the stale continuation", lambda: checker.check_transition_trace(commits, bad_stale, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "callee_tid": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M2 typed commit"))

    print("rtl m2 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

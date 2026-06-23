#!/usr/bin/env python3
"""Self-test the M16 typed endpoint commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m16_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m16_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M16 checker module")
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
    cf, cw, sf, sw, ops, backings = checker.load_schema()
    o = ops
    MEM = backings["LNP64_M16_BACKING_MEMORY"]
    REG = backings["LNP64_M16_BACKING_REGISTER"]
    OK, EAGAIN, EMSGSIZE, EBADF = checker.ERR_OK, checker.ERR_EAGAIN, checker.ERR_EMSGSIZE, checker.ERR_EBADF
    CAP = 2

    def commit(op, status, backing, bytes_len, caps_len, depth, caps_resolved, caps_installed):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "endpoint_id": 1,
                "endpoint_gen": 1, "backing": backing, "bytes_len": bytes_len, "caps_len": caps_len,
                "depth": depth, "capacity": CAP, "caps_resolved": caps_resolved,
                "caps_installed": caps_installed, "sender_domain_id": 1, "sender_domain_gen": 1,
                "receiver_domain_id": 2, "receiver_domain_gen": 1}

    flags = {f: 0 for f in sf if f not in ("op", "status", "depth", "capacity", "failures", "events")}
    flags["no_block_except_wait"] = 1
    failures = 0
    events = 0
    commits: list[dict] = []
    states: list[dict] = []

    def step(c, set_flags, fail_delta=0, event_delta=0):
        nonlocal failures, events
        commits.append(c)
        for k in set_flags:
            flags[k] = 1
        failures += fail_delta
        events += event_delta
        bounded = 1 if c["depth"] <= c["capacity"] else 0
        s = {"record": checker.STATE_NAME, "op": c["op"], "status": c["status"],
             "depth": c["depth"], "capacity": c["capacity"], "failures": failures, "events": events}
        s.update(flags)
        s["bounded_depth_le_capacity"] = bounded
        s["drain_bounded_by_capacity"] = 1
        s["counts_exact"] = 1 if (failures == 4 and events == 1) else 0
        states.append(s)

    step(commit(o["LNP64_M16_COMMIT_CREATE"], OK, MEM, 0, 0, 0, 0, 0), [])
    step(commit(o["LNP64_M16_COMMIT_SEND"], OK, MEM, 8, 0, 1, 0, 0), [])
    step(commit(o["LNP64_M16_COMMIT_RECV"], OK, MEM, 8, 0, 0, 0, 0), ["framing_one_send_one_recv"])
    step(commit(o["LNP64_M16_COMMIT_SEND"], OK, MEM, 8, 0, 1, 0, 0), [])
    step(commit(o["LNP64_M16_COMMIT_SEND"], OK, MEM, 8, 0, CAP, 0, 0), [])
    step(commit(o["LNP64_M16_COMMIT_SEND_FULL"], EAGAIN, MEM, 8, 0, CAP, 0, 0), ["full_fails_closed"], fail_delta=1)
    step(commit(o["LNP64_M16_COMMIT_RECV"], OK, MEM, 8, 0, 1, 0, 0), [])
    step(commit(o["LNP64_M16_COMMIT_RECV"], OK, MEM, 8, 0, 0, 0, 0), [])
    step(commit(o["LNP64_M16_COMMIT_RECV_EMPTY"], EAGAIN, MEM, 0, 0, 0, 0, 0), ["empty_fails_closed"], fail_delta=1)
    step(commit(o["LNP64_M16_COMMIT_OVERSIZE"], EMSGSIZE, MEM, 65, 0, 0, 0, 0), ["oversize_fails_closed"], fail_delta=1)
    step(commit(o["LNP64_M16_COMMIT_CAP_SEND"], OK, MEM, 8, 1, 1, 1, 1),
         ["caps_resolve_sender_only", "install_no_amplify"])
    step(commit(o["LNP64_M16_COMMIT_CAP_REJECT"], EBADF, MEM, 8, 1, 1, 0, 0),
         ["caps_reject_out_of_range"], fail_delta=1)
    step(commit(o["LNP64_M16_COMMIT_NOTIFY"], OK, REG, 0, 0, 1, 0, 0),
         ["notify_raises_register_edge"], event_delta=1)

    return commits, states, ops, backings, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, backings, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M16 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M16 state projection")
    check = lambda cm, st: checker.check_transition_trace(cm, st, ops, backings)
    check(commits, states)

    # op order must hold.
    scrambled = copy.deepcopy(commits)
    scrambled[1], scrambled[2] = scrambled[2], scrambled[1]
    expect_failure("sequence drifted", lambda: check(scrambled, states))

    # fail-closed: send on full must be an explicit EAGAIN.
    bad_full = copy.deepcopy(states)
    bad_full[5]["full_fails_closed"] = 0
    expect_failure("was not an explicit EAGAIN", lambda: check(commits, bad_full))

    # fail-closed: oversize must be EMSGSIZE.
    bad_oversize = copy.deepcopy(states)
    bad_oversize[9]["oversize_fails_closed"] = 0
    expect_failure("was not an explicit EMSGSIZE", lambda: check(commits, bad_oversize))

    # cap-safety: install must not amplify beyond resolve.
    bad_amplify = copy.deepcopy(commits)
    bad_amplify[10]["caps_installed"] = 5
    expect_failure("installed more caps than it resolved", lambda: check(bad_amplify, states))

    # cap-safety: a rejected cap must install nothing.
    bad_reject = copy.deepcopy(commits)
    bad_reject[11]["caps_installed"] = 1
    expect_failure("installed a rejected cap", lambda: check(bad_reject, states))

    # bounded: depth must never exceed capacity.
    bad_bounded = copy.deepcopy(commits)
    bad_bounded[4]["depth"] = 9
    expect_failure("depth exceeded capacity", lambda: check(bad_bounded, states))

    # an unbound endpoint must be rejected.
    bad_commit = copy.deepcopy(commits)
    bad_commit[0]["endpoint_gen"] = 0
    expect_failure("unbound endpoint", lambda: check(bad_commit, states))

    # terminal notify must see the prior invariants established.
    bad_terminal = copy.deepcopy(states)
    bad_terminal[12]["oversize_fails_closed"] = 0
    expect_failure("preceded invariant", lambda: check(commits, bad_terminal))

    bad_bits = list(commit_bits)
    bad_bits[1] = encode_bits({**commits[1], "bytes_len": 99}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M16 typed commit"))

    print("rtl m16 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Self-test the M5 typed DMA commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m5_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m5_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M5 checker module")
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
    RW, R = 3, 1

    def commit(op, status, gen, rights, dst_dom=1):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "src_buffer_id": 1,
                "dst_buffer_id": 2, "dst_generation": gen, "requester_domain": 1,
                "dst_domain": dst_dom, "dst_rights": rights}

    def state(op, status, gen, rights, pinned, completions, dst_dom=1, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "dst_buffer_id": 2, "dst_generation": gen,
                     "requester_domain": 1, "dst_domain": dst_dom, "dst_rights": rights,
                     "dst_pinned": pinned, "completions": completions,
                     "completions_exact": 1 if completions == 2 else 0})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M5_COMMIT_PIN"], checker.ERR_OK, 1, RW),
        commit(o["LNP64_M5_COMMIT_COPY"], checker.ERR_OK, 1, RW),
        commit(o["LNP64_M5_COMMIT_FILL"], checker.ERR_OK, 1, RW),
        commit(o["LNP64_M5_COMMIT_UNPIN"], checker.ERR_OK, 1, RW),
        commit(o["LNP64_M5_COMMIT_PERMISSION_FAULT"], checker.ERR_EACCES, 1, R),
        commit(o["LNP64_M5_COMMIT_REVOKED_SUBMIT"], checker.ERR_EREVOKED, 2, R),
        commit(o["LNP64_M5_COMMIT_DOMAIN_ISOLATION"], checker.ERR_EPERM, 2, R),
        commit(o["LNP64_M5_COMMIT_COHERENCE_FLUSH"], checker.ERR_OK, 2, R, dst_dom=2),
    ]
    states = [
        state(o["LNP64_M5_COMMIT_PIN"], checker.ERR_OK, 1, RW, 1, 0, pin_completed=1),
        state(o["LNP64_M5_COMMIT_COPY"], checker.ERR_OK, 1, RW, 1, 1, pin_completed=1, copy_completed=1),
        state(o["LNP64_M5_COMMIT_FILL"], checker.ERR_OK, 1, RW, 1, 2, pin_completed=1, copy_completed=1, fill_completed=1),
        state(o["LNP64_M5_COMMIT_UNPIN"], checker.ERR_OK, 1, RW, 0, 2, pin_completed=1, copy_completed=1, fill_completed=1, unpin_completed=1),
        state(o["LNP64_M5_COMMIT_PERMISSION_FAULT"], checker.ERR_EACCES, 1, R, 0, 2, pin_completed=1, copy_completed=1, fill_completed=1, unpin_completed=1, permission_faulted=1),
        state(o["LNP64_M5_COMMIT_REVOKED_SUBMIT"], checker.ERR_EREVOKED, 2, R, 0, 2, pin_completed=1, copy_completed=1, fill_completed=1, unpin_completed=1, permission_faulted=1, revoke_rejected=1),
        state(o["LNP64_M5_COMMIT_DOMAIN_ISOLATION"], checker.ERR_EPERM, 2, R, 0, 2, pin_completed=1, copy_completed=1, fill_completed=1, unpin_completed=1, permission_faulted=1, revoke_rejected=1, domain_isolation_enforced=1),
        state(o["LNP64_M5_COMMIT_COHERENCE_FLUSH"], checker.ERR_OK, 2, R, 0, 2, dst_dom=2, pin_completed=1, copy_completed=1, fill_completed=1, unpin_completed=1, permission_faulted=1, revoke_rejected=1, domain_isolation_enforced=1, coherence_observed=1, dst_visible=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M5 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M5 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # Copy against an unpinned buffer must fail (DMA confinement).
    bad_pin = copy.deepcopy(states)
    bad_pin[1]["dst_pinned"] = 0
    expect_failure("unpinned buffer", lambda: checker.check_transition_trace(commits, bad_pin, ops))

    # Copy crossing a domain boundary must fail.
    bad_dom = copy.deepcopy(commits)
    bad_dom[1]["dst_domain"] = 9
    expect_failure("crossed a domain boundary", lambda: checker.check_transition_trace(bad_dom, states, ops))

    # Permission fault that does not fault closed.
    bad_fault = copy.deepcopy(states)
    bad_fault[4]["permission_faulted"] = 0
    expect_failure("did not fault closed", lambda: checker.check_transition_trace(commits, bad_fault, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "dst_buffer_id": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M5 typed commit"))

    print("rtl m5 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Self-test the M6 typed service-boundary commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m6_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m6_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M6 checker module")
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

    def commit(op, status, cgen, sgen, req, ret):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "service_id": 1,
                "op_id": 1, "continuation_generation": cgen, "service_generation": sgen,
                "requested_rights": req, "returned_rights": ret}

    def state(op, status, cgen, sgen, installed, completions, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "continuation_generation": cgen,
                     "service_generation": sgen, "installed_caps": installed, "completions": completions})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M6_COMMIT_ENVELOPE"], checker.ERR_OK, 0, 1, 0, 0),
        commit(o["LNP64_M6_COMMIT_NS_DISPATCH"], checker.ERR_OK, 0, 1, 0, 0),
        commit(o["LNP64_M6_COMMIT_SERVICE_REQUEST"], checker.ERR_OK, 1, 1, 0, 0),
        commit(o["LNP64_M6_COMMIT_CAP_RETURN"], checker.ERR_OK, 1, 1, 3, 1),
        commit(o["LNP64_M6_COMMIT_SERVICE_CANCEL"], checker.ERR_ECANCELED, 1, 1, 0, 0),
        commit(o["LNP64_M6_COMMIT_STALE_SERVICE"], checker.ERR_EREVOKED, 1, 2, 0, 0),
        commit(o["LNP64_M6_COMMIT_CRASH_COMPLETION"], checker.ERR_EIO, 1, 2, 0, 0),
    ]
    states = [
        state(o["LNP64_M6_COMMIT_ENVELOPE"], checker.ERR_OK, 0, 1, 0, 0, envelope_validated=1),
        state(o["LNP64_M6_COMMIT_NS_DISPATCH"], checker.ERR_OK, 0, 1, 0, 0, envelope_validated=1, namespace_dispatched=1),
        state(o["LNP64_M6_COMMIT_SERVICE_REQUEST"], checker.ERR_OK, 1, 1, 0, 0, envelope_validated=1, namespace_dispatched=1, service_continuation_created=1),
        state(o["LNP64_M6_COMMIT_CAP_RETURN"], checker.ERR_OK, 1, 1, 1, 0, envelope_validated=1, namespace_dispatched=1, service_continuation_created=1, cap_return_installed=1, returned_cap_narrowed=1),
        state(o["LNP64_M6_COMMIT_SERVICE_CANCEL"], checker.ERR_ECANCELED, 1, 1, 1, 1, envelope_validated=1, namespace_dispatched=1, service_continuation_created=1, cap_return_installed=1, returned_cap_narrowed=1, cancel_terminal=1),
        state(o["LNP64_M6_COMMIT_STALE_SERVICE"], checker.ERR_EREVOKED, 1, 2, 1, 1, envelope_validated=1, namespace_dispatched=1, service_continuation_created=1, cap_return_installed=1, returned_cap_narrowed=1, cancel_terminal=1, stale_service_rejected=1),
        state(o["LNP64_M6_COMMIT_CRASH_COMPLETION"], checker.ERR_EIO, 1, 2, 1, 2, envelope_validated=1, namespace_dispatched=1, service_continuation_created=1, cap_return_installed=1, returned_cap_narrowed=1, cancel_terminal=1, stale_service_rejected=1, crash_completed=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M6 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M6 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # No authority amplification across the service boundary.
    amplify = copy.deepcopy(commits)
    amplify[3]["returned_rights"] = 0xF
    expect_failure("returned amplified rights", lambda: checker.check_transition_trace(amplify, states, ops))

    # Cap return that did not narrow.
    bad_narrow = copy.deepcopy(states)
    bad_narrow[3]["returned_cap_narrowed"] = 0
    expect_failure("did not narrow the returned cap", lambda: checker.check_transition_trace(commits, bad_narrow, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "service_id": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M6 typed commit"))

    print("rtl m6 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

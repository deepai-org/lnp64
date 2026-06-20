#!/usr/bin/env python3
"""Self-test the M9 typed classifier/servicelet commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m9_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m9_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M9 checker module")
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

    def commit(op, status, cyc_used, queue, mark):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "program_id": 1,
                "attachment_generation": 1, "cycle_budget": 16, "cycles_used": cyc_used,
                "queue_id": queue, "mark": mark}

    def state(op, status, packets, ipc, rejects, cyc_used, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "attachment_generation": 1, "packets": packets,
                     "ipc_records": ipc, "rejects": rejects, "cycle_budget": 16, "cycles_used": cyc_used})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M9_COMMIT_VERIFY_ACCEPT"], checker.ERR_OK, 0, 0, 0),
        commit(o["LNP64_M9_COMMIT_VERIFY_REJECT"], checker.ERR_EINVAL, 4, 0, 0),
        commit(o["LNP64_M9_COMMIT_PACKET_STEER"], checker.ERR_OK, 4, 7, 3),
        commit(o["LNP64_M9_COMMIT_IPC_STEER"], checker.ERR_OK, 4, 7, 3),
        commit(o["LNP64_M9_COMMIT_ACTION_EMIT"], checker.ERR_OK, 4, 7, 3),
        commit(o["LNP64_M9_COMMIT_BUDGET_EXHAUST"], checker.ERR_EAGAIN, 17, 0, 0),
        commit(o["LNP64_M9_COMMIT_STALE_ATTACHMENT"], checker.ERR_EREVOKED, 17, 0, 0),
    ]
    states = [
        state(o["LNP64_M9_COMMIT_VERIFY_ACCEPT"], checker.ERR_OK, 0, 0, 0, 4, verifier_accepted=1),
        state(o["LNP64_M9_COMMIT_VERIFY_REJECT"], checker.ERR_EINVAL, 0, 0, 1, 4, verifier_accepted=1, verifier_rejected=1),
        state(o["LNP64_M9_COMMIT_PACKET_STEER"], checker.ERR_OK, 1, 0, 1, 4, verifier_accepted=1, verifier_rejected=1, packet_steered=1),
        state(o["LNP64_M9_COMMIT_IPC_STEER"], checker.ERR_OK, 1, 1, 1, 4, verifier_accepted=1, verifier_rejected=1, packet_steered=1, ipc_steered=1),
        state(o["LNP64_M9_COMMIT_ACTION_EMIT"], checker.ERR_OK, 1, 1, 1, 4, verifier_accepted=1, verifier_rejected=1, packet_steered=1, ipc_steered=1, action_emitted=1, no_authority_created=1),
        state(o["LNP64_M9_COMMIT_BUDGET_EXHAUST"], checker.ERR_EAGAIN, 1, 1, 2, 17, verifier_accepted=1, verifier_rejected=1, packet_steered=1, ipc_steered=1, action_emitted=1, no_authority_created=1, budget_enforced=1, counts_exact=1),
        state(o["LNP64_M9_COMMIT_STALE_ATTACHMENT"], checker.ERR_EREVOKED, 1, 1, 2, 17, verifier_accepted=1, verifier_rejected=1, packet_steered=1, ipc_steered=1, action_emitted=1, no_authority_created=1, budget_enforced=1, counts_exact=1, stale_attachment_rejected=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M9 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M9 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # Servicelet action that created authority.
    bad_auth = copy.deepcopy(states)
    bad_auth[4]["no_authority_created"] = 0
    expect_failure("created authority", lambda: checker.check_transition_trace(commits, bad_auth, ops))

    # Verify reject not flagged.
    bad_reject = copy.deepcopy(states)
    bad_reject[1]["verifier_rejected"] = 0
    expect_failure("did not reject the malformed program", lambda: checker.check_transition_trace(commits, bad_reject, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "program_id": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M9 typed commit"))

    print("rtl m9 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

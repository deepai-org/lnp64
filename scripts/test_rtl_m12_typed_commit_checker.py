#!/usr/bin/env python3
"""Self-test the M12 typed storage-barrier commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m12_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m12_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M12 checker module")
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

    def commit(op, status, data_value=0):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "object_id": 1,
                "object_generation": 1, "domain_id": 1, "barrier_id": 1, "block_index": 0,
                "data_value": data_value}

    def state(op, status, completions, faults, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "completions": completions, "faults": faults,
                     "no_raw_device_authority": 1})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M12_COMMIT_BOOT_IMAGE"], checker.ERR_OK),
        commit(o["LNP64_M12_COMMIT_BLOCK_WRITE"], checker.ERR_OK, data_value=23063),
        commit(o["LNP64_M12_COMMIT_BARRIER"], checker.ERR_OK),
        commit(o["LNP64_M12_COMMIT_STALE_OBJECT"], checker.ERR_EREVOKED),
        commit(o["LNP64_M12_COMMIT_CROSS_DOMAIN"], checker.ERR_EPERM),
        commit(o["LNP64_M12_COMMIT_MEDIA_FAULT"], checker.ERR_EIO),
        commit(o["LNP64_M12_COMMIT_RAW_AUTHORITY"], checker.ERR_OK),
    ]
    done = dict(boot_image_visible=1, block_object_authorized=1, block_write_completed=1,
                storage_barrier_issued=1, storage_barrier_quiescent=1)
    states = [
        state(o["LNP64_M12_COMMIT_BOOT_IMAGE"], checker.ERR_OK, 1, 0, boot_image_visible=1),
        state(o["LNP64_M12_COMMIT_BLOCK_WRITE"], checker.ERR_OK, 2, 0, boot_image_visible=1, block_object_authorized=1, block_write_completed=1),
        state(o["LNP64_M12_COMMIT_BARRIER"], checker.ERR_OK, 3, 0, **done),
        state(o["LNP64_M12_COMMIT_STALE_OBJECT"], checker.ERR_EREVOKED, 3, 1, **done, stale_object_rejected=1),
        state(o["LNP64_M12_COMMIT_CROSS_DOMAIN"], checker.ERR_EPERM, 3, 2, **done, stale_object_rejected=1, cross_domain_rejected=1),
        state(o["LNP64_M12_COMMIT_MEDIA_FAULT"], checker.ERR_EIO, 3, 3, **done, stale_object_rejected=1, cross_domain_rejected=1, media_fault_terminal=1, counts_exact=1),
        state(o["LNP64_M12_COMMIT_RAW_AUTHORITY"], checker.ERR_OK, 3, 3, **done, stale_object_rejected=1, cross_domain_rejected=1, media_fault_terminal=1, counts_exact=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M12 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M12 state projection")
    check_transition = checker.check_transition_trace
    check_transition(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: check_transition(scrambled, states, ops))

    # A block write needs an authorized object.
    bad_auth = copy.deepcopy(states)
    bad_auth[1]["block_object_authorized"] = 0
    expect_failure("without an authorized object", lambda: check_transition(commits, bad_auth, ops))

    # Stale object must fail closed as revoked.
    bad_stale = copy.deepcopy(states)
    bad_stale[3]["stale_object_rejected"] = 0
    expect_failure("was not rejected as revoked", lambda: check_transition(commits, bad_stale, ops))

    # Cross-domain access must be denied.
    bad_cross = copy.deepcopy(states)
    bad_cross[4]["cross_domain_rejected"] = 0
    expect_failure("was not denied", lambda: check_transition(commits, bad_cross, ops))

    # The media fault must see the rejection invariants already established.
    bad_order = copy.deepcopy(states)
    bad_order[5]["cross_domain_rejected"] = 0
    expect_failure("preceded the rejection invariants", lambda: check_transition(commits, bad_order, ops))

    # Raw device authority must never be exposed in any projection.
    bad_raw = copy.deepcopy(states)
    bad_raw[0]["no_raw_device_authority"] = 0
    expect_failure("exposed raw device authority", lambda: check_transition(commits, bad_raw, ops))

    # An unbound object/domain must be rejected.
    bad_commit = copy.deepcopy(commits)
    bad_commit[0]["domain_id"] = 0
    expect_failure("unbound object/domain", lambda: check_transition(bad_commit, states, ops))

    bad_bits = list(commit_bits)
    bad_bits[1] = encode_bits({**commits[1], "data_value": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M12 typed commit"))

    print("rtl m12 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

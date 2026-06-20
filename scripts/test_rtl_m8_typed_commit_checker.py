#!/usr/bin/env python3
"""Self-test the M8 typed heap commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m8_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m8_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M8 checker module")
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

    def commit(op, status, pgen, size, ptr):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "owner_tid": 1,
                "pointer_generation": pgen, "heap_generation": 1, "size_class": size, "heap_ptr": ptr}

    def state(op, status, pgen, allocs, frees, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "owner_tid": 1, "pointer_generation": pgen,
                     "allocations": allocs, "frees": frees})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M8_COMMIT_ALLOC"], checker.ERR_OK, 1, 32, 4096),
        commit(o["LNP64_M8_COMMIT_ALLOC_SIZE"], checker.ERR_OK, 1, 32, 4096),
        commit(o["LNP64_M8_COMMIT_FREE"], checker.ERR_OK, 1, 32, 4096),
        commit(o["LNP64_M8_COMMIT_REUSE"], checker.ERR_OK, 2, 32, 4096),
        commit(o["LNP64_M8_COMMIT_DOUBLE_FREE"], checker.ERR_EINVAL, 2, 0, 0),
        commit(o["LNP64_M8_COMMIT_STALE_FREE"], checker.ERR_EREVOKED, 2, 0, 0),
        commit(o["LNP64_M8_COMMIT_CROSS_THREAD_FREE"], checker.ERR_OK, 2, 0, 0),
        commit(o["LNP64_M8_COMMIT_GUARD_FAULT"], checker.ERR_EFAULT, 2, 0, 0),
    ]
    states = [
        state(o["LNP64_M8_COMMIT_ALLOC"], checker.ERR_OK, 1, 1, 0, allocated=1, alloc_completed=1),
        state(o["LNP64_M8_COMMIT_ALLOC_SIZE"], checker.ERR_OK, 1, 1, 0, allocated=1, alloc_completed=1, alloc_size_reported=1),
        state(o["LNP64_M8_COMMIT_FREE"], checker.ERR_OK, 2, 1, 1, quarantined=1, alloc_completed=1, alloc_size_reported=1, free_completed=1, quarantine_observed=1),
        state(o["LNP64_M8_COMMIT_REUSE"], checker.ERR_OK, 2, 2, 1, allocated=1, alloc_completed=1, alloc_size_reported=1, free_completed=1, quarantine_observed=1, reuse_completed=1),
        state(o["LNP64_M8_COMMIT_DOUBLE_FREE"], checker.ERR_EINVAL, 2, 2, 1, allocated=1, alloc_completed=1, alloc_size_reported=1, free_completed=1, quarantine_observed=1, reuse_completed=1, double_free_rejected=1),
        state(o["LNP64_M8_COMMIT_STALE_FREE"], checker.ERR_EREVOKED, 2, 2, 1, allocated=1, alloc_completed=1, alloc_size_reported=1, free_completed=1, quarantine_observed=1, reuse_completed=1, double_free_rejected=1, stale_pointer_rejected=1),
        state(o["LNP64_M8_COMMIT_CROSS_THREAD_FREE"], checker.ERR_OK, 2, 2, 2, quarantined=1, alloc_completed=1, alloc_size_reported=1, free_completed=1, quarantine_observed=1, reuse_completed=1, double_free_rejected=1, stale_pointer_rejected=1, cross_thread_handoff=1, heap_count_exact=1),
        state(o["LNP64_M8_COMMIT_GUARD_FAULT"], checker.ERR_EFAULT, 2, 2, 2, quarantined=1, alloc_completed=1, alloc_size_reported=1, free_completed=1, quarantine_observed=1, reuse_completed=1, double_free_rejected=1, stale_pointer_rejected=1, cross_thread_handoff=1, heap_count_exact=1, guard_faulted=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M8 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M8 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # Double free not rejected.
    bad_df = copy.deepcopy(states)
    bad_df[4]["double_free_rejected"] = 0
    expect_failure("did not reject the double free", lambda: checker.check_transition_trace(commits, bad_df, ops))

    # Stale free not rejected.
    bad_stale = copy.deepcopy(states)
    bad_stale[5]["stale_pointer_rejected"] = 0
    expect_failure("did not reject the stale pointer", lambda: checker.check_transition_trace(commits, bad_stale, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "owner_tid": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M8 typed commit"))

    print("rtl m8 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

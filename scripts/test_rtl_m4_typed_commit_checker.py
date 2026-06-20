#!/usr/bin/env python3
"""Self-test the M4 typed VMA/MMU commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m4_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m4_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M4 checker module")
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
    PERM_RX, PERM_R = 5, 1

    def commit(op, status, gen, perms, faddr):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "vma_id": 1,
                "vma_generation": gen, "permissions": perms, "fault_addr": faddr}

    def state(op, status, gen, perms, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "vma_id": 1, "vma_generation": gen,
                     "permissions": perms, "guard_page_valid": 1, "tlb_valid": 1,
                     "mapping_created": 1, "wx_enforced": 1})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M4_COMMIT_MMAP"], checker.ERR_OK, 1, PERM_RX, 0),
        commit(o["LNP64_M4_COMMIT_LOAD"], checker.ERR_OK, 1, PERM_RX, 16384),
        commit(o["LNP64_M4_COMMIT_STORE_DENIED"], checker.ERR_EACCES, 1, PERM_RX, 0),
        commit(o["LNP64_M4_COMMIT_EXEC_FAULT"], checker.ERR_EFAULT, 1, PERM_R, 0),
        commit(o["LNP64_M4_COMMIT_GUARD_FAULT"], checker.ERR_EFAULT, 1, PERM_R, 0),
        commit(o["LNP64_M4_COMMIT_STALE_REJECT"], checker.ERR_EREVOKED, 2, PERM_R, 0),
        commit(o["LNP64_M4_COMMIT_TLB_INVALIDATE"], checker.ERR_OK, 2, PERM_R, 0),
    ]
    states = [
        state(o["LNP64_M4_COMMIT_MMAP"], checker.ERR_OK, 1, PERM_RX),
        state(o["LNP64_M4_COMMIT_LOAD"], checker.ERR_OK, 1, PERM_RX, load_permitted=1),
        state(o["LNP64_M4_COMMIT_STORE_DENIED"], checker.ERR_EACCES, 1, PERM_RX, load_permitted=1, store_rejected=1),
        state(o["LNP64_M4_COMMIT_EXEC_FAULT"], checker.ERR_EFAULT, 1, PERM_R, load_permitted=1, store_rejected=1, nx_faulted=1),
        state(o["LNP64_M4_COMMIT_GUARD_FAULT"], checker.ERR_EFAULT, 1, PERM_R, load_permitted=1, store_rejected=1, nx_faulted=1, guard_faulted=1),
        state(o["LNP64_M4_COMMIT_STALE_REJECT"], checker.ERR_EREVOKED, 2, PERM_R, load_permitted=1, store_rejected=1, nx_faulted=1, guard_faulted=1, stale_vma_rejected=1),
        state(o["LNP64_M4_COMMIT_TLB_INVALIDATE"], checker.ERR_OK, 2, PERM_R, tlb_valid=0, load_permitted=1, store_rejected=1, nx_faulted=1, guard_faulted=1, stale_vma_rejected=1, tlb_invalidation_observed=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    # Positive: bits decode and the transition trace checks out.
    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M4 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M4 state projection")
    checker.check_transition_trace(commits, states, ops)

    # Wrong op sequence.
    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # W^X invariant violated.
    bad_wx = copy.deepcopy(states)
    bad_wx[0]["wx_enforced"] = 0
    expect_failure("W^X invariant", lambda: checker.check_transition_trace(commits, bad_wx, ops))

    # Store-denied without the rejection flag.
    bad_store = copy.deepcopy(states)
    bad_store[2]["store_rejected"] = 0
    expect_failure("did not reject the write", lambda: checker.check_transition_trace(commits, bad_store, ops))

    # Packed bit drift.
    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "vma_id": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M4 typed commit"))

    print("rtl m4 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

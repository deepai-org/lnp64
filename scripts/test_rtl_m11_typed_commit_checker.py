#!/usr/bin/env python3
"""Self-test the M11 typed DDR/metadata commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m11_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m11_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M11 checker module")
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
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "line_id": 1,
                "line_generation": 1, "domain_id": 1, "metadata_epoch": 1, "byte_len": 64,
                "data_value": data_value}

    def state(op, status, completions, faults, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "completions": completions, "faults": faults})
        base.update(flags)
        return base

    o = ops
    bound = dict(metadata_allocated=1, metadata_domain_bound=1)
    commits = [
        commit(o["LNP64_M11_COMMIT_METADATA_ALLOC"], checker.ERR_OK),
        commit(o["LNP64_M11_COMMIT_DDR_WRITE"], checker.ERR_OK, data_value=4660),
        commit(o["LNP64_M11_COMMIT_DDR_READ"], checker.ERR_OK, data_value=4660),
        commit(o["LNP64_M11_COMMIT_STALE_SUBMIT"], checker.ERR_EREVOKED),
        commit(o["LNP64_M11_COMMIT_CROSS_DOMAIN"], checker.ERR_EPERM),
        commit(o["LNP64_M11_COMMIT_ECC_SCRUB"], checker.ERR_EIO),
        commit(o["LNP64_M11_COMMIT_BARRIER"], checker.ERR_OK),
    ]
    states = [
        state(o["LNP64_M11_COMMIT_METADATA_ALLOC"], checker.ERR_OK, 0, 0, **bound),
        state(o["LNP64_M11_COMMIT_DDR_WRITE"], checker.ERR_OK, 1, 0, **bound, ddr_write_completed=1),
        state(o["LNP64_M11_COMMIT_DDR_READ"], checker.ERR_OK, 2, 0, **bound, ddr_write_completed=1, ddr_read_completed=1, read_matches_write=1),
        state(o["LNP64_M11_COMMIT_STALE_SUBMIT"], checker.ERR_EREVOKED, 2, 1, **bound, ddr_write_completed=1, ddr_read_completed=1, read_matches_write=1, stale_generation_rejected=1),
        state(o["LNP64_M11_COMMIT_CROSS_DOMAIN"], checker.ERR_EPERM, 2, 2, **bound, ddr_write_completed=1, ddr_read_completed=1, read_matches_write=1, stale_generation_rejected=1, cross_domain_rejected=1),
        state(o["LNP64_M11_COMMIT_ECC_SCRUB"], checker.ERR_EIO, 2, 3, **bound, ddr_write_completed=1, ddr_read_completed=1, read_matches_write=1, stale_generation_rejected=1, cross_domain_rejected=1, ecc_scrubbed=1, counts_exact=1),
        state(o["LNP64_M11_COMMIT_BARRIER"], checker.ERR_OK, 2, 3, **bound, ddr_write_completed=1, ddr_read_completed=1, read_matches_write=1, stale_generation_rejected=1, cross_domain_rejected=1, ecc_scrubbed=1, barrier_quiescent=1, counts_exact=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M11 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M11 state projection")
    check_transition = checker.check_transition_trace
    check_transition(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: check_transition(scrambled, states, ops))

    # DDR read must reproduce the prior write (SG-MEM read-after-write).
    bad_read = copy.deepcopy(states)
    bad_read[2]["read_matches_write"] = 0
    expect_failure("did not match the prior write", lambda: check_transition(commits, bad_read, ops))

    # Stale generation must fail closed as revoked.
    bad_stale = copy.deepcopy(states)
    bad_stale[3]["stale_generation_rejected"] = 0
    expect_failure("was not rejected as revoked", lambda: check_transition(commits, bad_stale, ops))

    # Cross-domain access must be denied.
    bad_cross = copy.deepcopy(states)
    bad_cross[4]["cross_domain_rejected"] = 0
    expect_failure("was not denied", lambda: check_transition(commits, bad_cross, ops))

    # The scrub must see the rejection invariants already established.
    bad_order = copy.deepcopy(states)
    bad_order[5]["cross_domain_rejected"] = 0
    expect_failure("preceded the rejection invariants", lambda: check_transition(commits, bad_order, ops))

    # An unbound line/domain must be rejected.
    bad_commit = copy.deepcopy(commits)
    bad_commit[0]["domain_id"] = 0
    expect_failure("unbound line/domain", lambda: check_transition(bad_commit, states, ops))

    bad_bits = list(commit_bits)
    bad_bits[1] = encode_bits({**commits[1], "data_value": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M11 typed commit"))

    print("rtl m11 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

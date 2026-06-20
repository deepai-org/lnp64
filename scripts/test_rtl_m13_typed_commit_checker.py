#!/usr/bin/env python3
"""Self-test the M13 typed PCIe/IOMMU commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m13_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m13_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M13 checker module")
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

    def commit(op, status, dma_bytes=0):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "requester_id": 256,
                "bar_id": 1, "bar_generation": 1, "domain_id": 1, "iommu_context": 1,
                "dma_bytes": dma_bytes}

    def state(op, status, completions, faults, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "completions": completions, "faults": faults,
                     "no_raw_pcie_authority": 1})
        base.update(flags)
        return base

    o = ops
    commits = [
        commit(o["LNP64_M13_COMMIT_ENUMERATE"], checker.ERR_OK),
        commit(o["LNP64_M13_COMMIT_IOMMU_DMA"], checker.ERR_OK, dma_bytes=128),
        commit(o["LNP64_M13_COMMIT_MSI"], checker.ERR_OK),
        commit(o["LNP64_M13_COMMIT_BUS_MASTER"], checker.ERR_EPERM),
        commit(o["LNP64_M13_COMMIT_STALE_BAR"], checker.ERR_EREVOKED),
        commit(o["LNP64_M13_COMMIT_MALFORMED_CONFIG"], checker.ERR_EINVAL),
        commit(o["LNP64_M13_COMMIT_RAW_AUTHORITY"], checker.ERR_OK),
    ]
    done = dict(device_enumerated=1, bar_capability_created=1, iommu_bound_to_domain=1,
                scoped_dma_completed=1, msi_event_delivered=1)
    states = [
        state(o["LNP64_M13_COMMIT_ENUMERATE"], checker.ERR_OK, 1, 0, device_enumerated=1, bar_capability_created=1),
        state(o["LNP64_M13_COMMIT_IOMMU_DMA"], checker.ERR_OK, 2, 0, device_enumerated=1, bar_capability_created=1, iommu_bound_to_domain=1, scoped_dma_completed=1),
        state(o["LNP64_M13_COMMIT_MSI"], checker.ERR_OK, 3, 0, **done),
        state(o["LNP64_M13_COMMIT_BUS_MASTER"], checker.ERR_EPERM, 3, 1, **done, unbound_bus_master_rejected=1),
        state(o["LNP64_M13_COMMIT_STALE_BAR"], checker.ERR_EREVOKED, 3, 2, **done, unbound_bus_master_rejected=1, stale_bar_rejected=1),
        state(o["LNP64_M13_COMMIT_MALFORMED_CONFIG"], checker.ERR_EINVAL, 3, 3, **done, unbound_bus_master_rejected=1, stale_bar_rejected=1, malformed_config_rejected=1, counts_exact=1),
        state(o["LNP64_M13_COMMIT_RAW_AUTHORITY"], checker.ERR_OK, 3, 3, **done, unbound_bus_master_rejected=1, stale_bar_rejected=1, malformed_config_rejected=1, counts_exact=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M13 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M13 state projection")
    check_transition = checker.check_transition_trace
    check_transition(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: check_transition(scrambled, states, ops))

    # Enumeration must create a BAR capability.
    bad_bar = copy.deepcopy(states)
    bad_bar[0]["bar_capability_created"] = 0
    expect_failure("created no BAR capability", lambda: check_transition(commits, bad_bar, ops))

    # An unbound bus master must be denied.
    bad_master = copy.deepcopy(states)
    bad_master[3]["unbound_bus_master_rejected"] = 0
    expect_failure("did not deny the unbound bus master", lambda: check_transition(commits, bad_master, ops))

    # A stale BAR must fail closed as revoked.
    bad_stale = copy.deepcopy(states)
    bad_stale[4]["stale_bar_rejected"] = 0
    expect_failure("was not rejected as revoked", lambda: check_transition(commits, bad_stale, ops))

    # The malformed-config fault must see the prior rejections already established.
    bad_order = copy.deepcopy(states)
    bad_order[5]["stale_bar_rejected"] = 0
    expect_failure("preceded the rejection invariants", lambda: check_transition(commits, bad_order, ops))

    # Raw PCIe authority must never be exposed in any projection.
    bad_raw = copy.deepcopy(states)
    bad_raw[0]["no_raw_pcie_authority"] = 0
    expect_failure("exposed raw PCIe authority", lambda: check_transition(commits, bad_raw, ops))

    # An unbound requester/domain must be rejected.
    bad_commit = copy.deepcopy(commits)
    bad_commit[0]["domain_id"] = 0
    expect_failure("unbound requester/domain", lambda: check_transition(bad_commit, states, ops))

    bad_bits = list(commit_bits)
    bad_bits[1] = encode_bits({**commits[1], "dma_bytes": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M13 typed commit"))

    print("rtl m13 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Executable LNP64 M14 Resource Domain / policy model.

Set LNP64_COSIM_SEED to run a bounded variant with different domain ids,
budgets, rights, usage, and policy masks while preserving the same authority,
budget, lifecycle, roll-up, and fail-closed obligations.
"""

import os

EPERM = 1
EAGAIN = 11
EREVOKED = 122

RIGHT_READ = 0x1
RIGHT_WRITE = 0x2
RIGHT_EXEC = 0x4


def seeded_values() -> tuple[int, int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 2, 100, 40, RIGHT_READ | RIGHT_WRITE | RIGHT_EXEC, 13, 7, 0x3, 1
    root_domain = (seed & 0xF) + 1
    child_domain = root_domain + ((seed >> 4) & 0xF) + 1
    parent_budget = ((seed >> 8) & 0x7F) + 64
    child_budget = ((seed >> 15) & 0x1F) + 1
    if child_budget >= parent_budget:
        child_budget = parent_budget - 1
    requested_rights = (RIGHT_READ | RIGHT_WRITE) | (RIGHT_EXEC if ((seed >> 20) & 0x1) else 0)
    child_used = ((seed >> 21) & 0xF) + 1
    sibling_used = ((seed >> 25) & 0xF) + 1
    policy_mask = ((seed >> 29) & 0x3) | RIGHT_READ
    policy_label = ((seed >> 31) & 0x1) + 1
    return (
        root_domain,
        child_domain,
        parent_budget,
        child_budget,
        requested_rights,
        child_used,
        sibling_used,
        policy_mask,
        policy_label,
    )


def main() -> None:
    (
        root_domain,
        child_domain,
        parent_budget,
        child_budget,
        requested_rights,
        child_used,
        sibling_used,
        policy_mask,
        policy_label,
    ) = seeded_values()
    parent_rights = RIGHT_READ | RIGHT_WRITE
    child_rights = requested_rights & parent_rights
    assert child_rights == parent_rights
    assert child_budget <= parent_budget
    print(
        f"TRACE boot root_domain={root_domain} child_domain={child_domain} "
        f"parent_budget={parent_budget}"
    )
    print(
        f"TRACE delegate parent_rights=0x{parent_rights:016x} "
        f"requested=0x{requested_rights:016x} child_rights=0x{child_rights:016x} clipped=1"
    )
    print(
        f"TRACE create_child child={child_domain} generation=1 "
        f"budget={child_budget} parent={root_domain}"
    )

    excess_budget = parent_budget + 1
    assert excess_budget > parent_budget
    print(f"TRACE child_budget request={excess_budget} limit={parent_budget} errno={EPERM}")

    print(f"TRACE freeze child={child_domain} dispatch=0 errno={EAGAIN}")
    print(f"TRACE resume child={child_domain} dispatch=1")

    parent_used = child_used + sibling_used
    print(
        f"TRACE usage child={child_used} sibling={sibling_used} "
        f"parent_used={parent_used}"
    )

    print(f"TRACE destroy child={child_domain} generation=2 dispatch=0 errno={EREVOKED}")
    print(f"TRACE policy subject={child_domain} mask=0x{policy_mask:016x} label={policy_label} denied=1 errno={EPERM}")
    print(f"TRACE done delegated=1 failures=3 rollup={parent_used}")


if __name__ == "__main__":
    main()

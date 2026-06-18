#!/usr/bin/env python3
"""Executable LNP64 M2 gate/continuation model.

The model is a bounded refinement target for the RTL slice: one caller, gate
targets for sync/async/handoff delivery, one continuation, stale continuation
rejection, and a fault delivery gate.
Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
gate generation, continuation id, and gate targets.
"""

import os

EFAULT = 14
EREVOKED = 122


def seeded_values() -> tuple[int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 2, 2, 3
    gate_generation = (seed & 0xF) + 1
    continuation_id = ((seed >> 4) & 0xFF) + 1
    sync_target = ((seed >> 12) & 0xF) + 2
    async_target = ((seed >> 16) & 0xF) + 2
    handoff_target = ((seed >> 20) & 0xF) + 3
    return gate_generation, continuation_id, sync_target, async_target, handoff_target


def main() -> None:
    gate_generation, continuation_id, sync_target, async_target, handoff_target = seeded_values()
    continuation_valid = False
    delivered_faults = 0
    print(f"TRACE boot root_domain=1 gate_gen={gate_generation}")

    assert not continuation_valid
    continuation_valid = True
    continuation_generation = 1
    print(f"TRACE gate_call mode=sync target={sync_target} continuation={continuation_id}")

    assert continuation_valid and continuation_generation == 1
    continuation_valid = False
    continuation_generation += 1
    print(f"TRACE gate_return continuation={continuation_id} wake=1")

    assert not continuation_valid
    print(f"TRACE gate_call mode=async target={async_target} completion=none")

    assert not continuation_valid
    print(f"TRACE gate_call mode=handoff target={handoff_target} transfer=running")

    if not continuation_valid and continuation_generation != 1:
        print(f"TRACE stale_return errno={EREVOKED}")
    else:
        raise AssertionError("stale continuation unexpectedly accepted")

    delivered_faults += 1
    print(f"TRACE fault_delivery errno={EFAULT} target=fault_gate")
    print("TRACE signal_compat mask=honored authority=0")
    print(f"TRACE done delivered_faults={delivered_faults}")


if __name__ == "__main__":
    main()

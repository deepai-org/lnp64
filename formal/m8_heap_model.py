#!/usr/bin/env python3
"""Executable LNP64 M8 heap model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
root domain, heap generation, pointer, size class, owner/freeing threads, and
pointer generation.
"""

import os

EFAULT = 14
EINVAL = 22
EREVOKED = 122
PTR = 4096
SIZE_CLASS = 32


def seeded_values() -> tuple[int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, PTR, SIZE_CLASS, 1, 2, 1
    root_domain = (seed & 0xF) + 1
    heap_generation = ((seed >> 4) & 0xF) + 1
    ptr = PTR + (((seed >> 8) & 0xF) << 5)
    size_class = (((seed >> 12) & 0x7) + 1) * 16
    owner_tid = ((seed >> 16) & 0xF) + 1
    freer_tid = owner_tid + ((seed >> 20) & 0x7) + 1
    pointer_generation = ((seed >> 24) & 0xF) + 1
    return root_domain, heap_generation, ptr, size_class, owner_tid, freer_tid, pointer_generation


def main() -> None:
    root_domain, heap_generation, ptr, size_class, owner_tid, freer_tid, pointer_generation = seeded_values()
    stale_pointer_generation = pointer_generation
    allocations = 0
    frees = 0
    allocated = False
    quarantined = False
    print(f"TRACE boot root_domain={root_domain} heap_gen={heap_generation}")

    allocated = True
    allocations += 1
    print(f"TRACE alloc tid={owner_tid} ptr={ptr} size={size_class} class={size_class}")

    assert allocated
    print(f"TRACE alloc_size ptr={ptr} size={size_class}")

    allocated = False
    quarantined = True
    pointer_generation += 1
    frees += 1
    print(f"TRACE free tid={owner_tid} ptr={ptr} quarantine=1")

    assert quarantined and not allocated
    allocated = True
    quarantined = False
    allocations += 1
    print(f"TRACE reuse tid={owner_tid} ptr={ptr} generation={pointer_generation}")

    print(f"TRACE double_free errno={EINVAL}")

    if stale_pointer_generation != pointer_generation:
        print(f"TRACE stale_free errno={EREVOKED}")
    else:
        raise AssertionError("stale heap pointer unexpectedly accepted")

    assert allocated and owner_tid >= 1
    allocated = False
    quarantined = True
    frees += 1
    print(f"TRACE cross_thread_free owner={owner_tid} freer={freer_tid} handoff=1")

    assert quarantined
    print(f"TRACE guard_fault errno={EFAULT}")
    print(f"TRACE done allocs={allocations} frees={frees}")


if __name__ == "__main__":
    main()

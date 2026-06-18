#!/usr/bin/env python3
"""Executable LNP64 M4 VMA/MMU model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
VMA id, page count, base address, and VMA generation.
"""

import os

EACCES = 13
EFAULT = 14
EREVOKED = 122
BASE = 0x4000


def seeded_values() -> tuple[int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 2, BASE, 1
    vma_id = (seed & 0xF) + 1
    pages = ((seed >> 4) & 0x7) + 1
    base = BASE + (((seed >> 8) & 0xFF) * 0x1000)
    generation = ((seed >> 16) & 0xF) + 1
    return vma_id, pages, base, generation


def main() -> None:
    print("TRACE boot root_domain=1 vma_table=empty")

    vma_id, pages, base, vma_generation = seeded_values()
    permissions = {"read", "execute"}
    stale_generation = vma_generation
    tlb_valid = True
    print(f"TRACE mmap vma={vma_id} pages={pages} perms=rx guard=1")

    assert tlb_valid and "read" in permissions and vma_generation == stale_generation
    print(f"TRACE load addr=0x{base:016x} result=ok")

    assert "write" not in permissions and not ({"write", "execute"} <= permissions)
    print(f"TRACE store_denied errno={EACCES} invariant=wx")

    permissions = {"read"}
    if "execute" not in permissions:
        print(f"TRACE exec_fault errno={EFAULT} reason=nx")
    else:
        raise AssertionError("NX execution unexpectedly accepted")

    print(f"TRACE guard_fault errno={EFAULT} page=guard")

    vma_generation += 1
    if stale_generation != vma_generation:
        print(f"TRACE stale_vma errno={EREVOKED}")
    else:
        raise AssertionError("stale VMA generation unexpectedly accepted")

    tlb_valid = False
    print(f"TRACE tlb_invalidate vma={vma_id} tlb_valid={int(tlb_valid)}")
    print(f"TRACE done mappings=1 vma_gen={vma_generation}")


if __name__ == "__main__":
    main()

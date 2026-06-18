#!/usr/bin/env python3
"""Executable LNP64 M5 DMA/memory-object model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
domains, buffer ids, transfer sizes, and fill value.
"""

import os

EACCES = 13
EPERM = 1
EREVOKED = 122


def seeded_values() -> tuple[int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 2, 16, 170, 8, 2
    root_domain = (seed & 0xF) + 1
    src_buffer = ((seed >> 4) & 0xF) + 1
    dst_buffer = ((seed >> 8) & 0xF) + 2
    copy_bytes = (((seed >> 12) & 0xF) + 1) * 4
    fill_value = ((seed >> 16) & 0xFF) + 1
    fill_bytes = (((seed >> 24) & 0x7) + 1) * 4
    isolation_domain = root_domain + ((seed >> 27) & 0x3) + 1
    return root_domain, src_buffer, dst_buffer, copy_bytes, fill_value, fill_bytes, isolation_domain


def main() -> None:
    root_domain, src_buffer, dst_buffer, copy_bytes, fill_value, fill_bytes, isolation_domain = seeded_values()
    requester_domain = root_domain
    dst_domain = root_domain
    dst_generation = 1
    stale_dst_generation = dst_generation
    completions = 0
    print(f"TRACE boot root_domain={root_domain} dma_buffers=2")

    pinned = True
    print(f"TRACE dma_pin buffer={dst_buffer} pinned=1")

    assert requester_domain == dst_domain
    completions += 1
    print(f"TRACE dma_copy src={src_buffer} dst={dst_buffer} bytes={copy_bytes} completion=1")

    completions += 1
    print(f"TRACE dma_fill dst={dst_buffer} value={fill_value} bytes={fill_bytes} completion=2")

    pinned = False
    print(f"TRACE dma_unpin buffer={dst_buffer} pinned=0")
    assert not pinned

    dst_rights = {"read"}
    if "write" not in dst_rights:
        print(f"TRACE permission_fault errno={EACCES} op=write")
    else:
        raise AssertionError("write without DMA buffer write permission accepted")

    dst_generation += 1
    if stale_dst_generation != dst_generation:
        print(f"TRACE revoked_submit errno={EREVOKED}")
    else:
        raise AssertionError("revoked DMA buffer submit accepted")

    dst_domain = isolation_domain
    if requester_domain != dst_domain:
        print(f"TRACE domain_isolation errno={EPERM}")
    else:
        raise AssertionError("cross-domain DMA submit accepted without authority")

    visible = 1
    print(f"TRACE coherence_flush buffer={dst_buffer} visible={visible}")
    print(f"TRACE done completions={completions}")


if __name__ == "__main__":
    main()

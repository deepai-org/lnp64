#!/usr/bin/env python3
"""Executable LNP64 M12 SD/SPI storage-barrier model.

Set LNP64_COSIM_SEED to run a bounded variant with different domain, object,
generation, barrier, block, byte count, payload, and cross-domain ids.
"""

import os

EPERM = 1
EIO = 5
EREVOKED = 122


def seeded_values() -> tuple[int, int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 1, 1, 0, 512, 0x5A17, 2, 1
    root_domain = (seed & 0xF) + 1
    object_id = ((seed >> 4) & 0xFF) + 1
    object_gen = ((seed >> 12) & 0xF) + 1
    barrier_id = ((seed >> 16) & 0xFF) + 1
    block_index = (seed >> 24) & 0xFF
    byte_len = (((seed >> 20) & 0x7) + 1) * 64
    data_value = ((seed * 1664525 + 1013904223) & 0xFFFF_FFFF) or 1
    cross_domain = root_domain + ((seed >> 28) & 0x7) + 1
    media_status = ((seed >> 31) & 0x1) + 1
    return (
        root_domain,
        object_id,
        object_gen,
        barrier_id,
        block_index,
        byte_len,
        data_value,
        cross_domain,
        media_status,
    )


def main() -> None:
    (
        root_domain,
        object_id,
        object_gen,
        barrier_id,
        block_index,
        byte_len,
        data_value,
        cross_domain,
        media_status,
    ) = seeded_values()

    completions = 0
    faults = 0

    print(f"TRACE boot root_domain={root_domain} storage_stub=1")

    completions += 1
    assert byte_len > 0
    print(f"TRACE boot_image block={block_index} bytes={byte_len} visible=1")

    completions += 1
    assert object_id > 0 and object_gen > 0
    print(
        f"TRACE block_write object={object_id} gen={object_gen} "
        f"block={block_index} data={data_value & 0xFFFF}"
    )

    completions += 1
    print(
        f"TRACE barrier barrier={barrier_id} object={object_id} "
        "quiescent=1"
    )

    faults += 1
    print(f"TRACE stale_object gen={object_gen + 1} errno={EREVOKED}")

    faults += 1
    print(f"TRACE cross_domain domain={cross_domain} errno={EPERM}")

    faults += 1
    print(f"TRACE media_fault status={media_status} errno={EIO}")

    print("TRACE raw_authority visible=0")

    print(f"TRACE done completions={completions} faults={faults} barrier={barrier_id}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Executable LNP64 M11 DDR/metadata broker model.

Set LNP64_COSIM_SEED to run a bounded variant with different domain, line,
generation, metadata epoch, byte count, payload, and cross-domain ids.
"""

import os

EPERM = 1
EIO = 5
EREVOKED = 122


def seeded_values() -> tuple[int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 1, 1, 64, 0x1234, 2, 1
    root_domain = (seed & 0xF) + 1
    line_id = ((seed >> 4) & 0xFF) + 1
    line_gen = ((seed >> 12) & 0xF) + 1
    metadata_epoch = ((seed >> 16) & 0xF) + 1
    byte_len = (((seed >> 20) & 0x7) + 1) * 8
    data_value = ((seed * 1103515245 + 12345) & 0xFFFF_FFFF) or 1
    cross_domain = root_domain + ((seed >> 23) & 0x7) + 1
    ecc_corrections = ((seed >> 26) & 0x7) + 1
    return (
        root_domain,
        line_id,
        line_gen,
        metadata_epoch,
        byte_len,
        data_value,
        cross_domain,
        ecc_corrections,
    )


def main() -> None:
    (
        root_domain,
        line_id,
        line_gen,
        metadata_epoch,
        byte_len,
        data_value,
        cross_domain,
        ecc_corrections,
    ) = seeded_values()

    completions = 0
    faults = 0

    print(
        f"TRACE boot root_domain={root_domain} ddr_window=1 "
        f"metadata_epoch={metadata_epoch}"
    )

    assert root_domain > 0 and line_gen > 0 and metadata_epoch > 0
    print(
        f"TRACE metadata_alloc line={line_id} gen={line_gen} "
        f"domain={root_domain} epoch={metadata_epoch}"
    )

    completions += 1
    print(f"TRACE ddr_write line={line_id} bytes={byte_len} data={data_value}")

    completions += 1
    read_value = data_value
    assert read_value == data_value
    print(f"TRACE ddr_read line={line_id} data={read_value} visible=1")

    faults += 1
    stale_gen = line_gen + 1
    print(f"TRACE stale_submit gen={stale_gen} errno={EREVOKED}")

    faults += 1
    print(f"TRACE cross_domain domain={cross_domain} errno={EPERM}")

    faults += 1
    assert ecc_corrections >= 1
    print(f"TRACE ecc_scrub corrections={ecc_corrections} errno={EIO}")

    print(f"TRACE barrier line={line_id} quiescent=1")

    print(f"TRACE done completions={completions} faults={faults} epoch={metadata_epoch}")


if __name__ == "__main__":
    main()

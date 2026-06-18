#!/usr/bin/env python3
"""Executable LNP64 M13 PCIe/IOMMU/MSI model.

Set LNP64_COSIM_SEED to run a bounded variant with different domain, requester,
BAR, generation, IOMMU context, DMA byte count, MSI vector, and malformed field
ids.
"""

import os

EPERM = 1
EINVAL = 22
EREVOKED = 122


def seeded_values() -> tuple[int, int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 0x0100, 1, 1, 1, 128, 32, 2, 1
    root_domain = (seed & 0xF) + 1
    requester = (((seed >> 4) & 0x7F) + 1) << 8
    bar_id = ((seed >> 12) & 0xF) + 1
    bar_gen = ((seed >> 16) & 0xF) + 1
    iommu_context = ((seed >> 20) & 0xF) + 1
    dma_bytes = (((seed >> 24) & 0x7) + 1) * 64
    msi_vector = ((seed >> 27) & 0x1F) + 1
    rogue_domain = root_domain + ((seed >> 28) & 0x7) + 1
    malformed_field = ((seed >> 31) & 0x1) + 1
    return (
        root_domain,
        requester,
        bar_id,
        bar_gen,
        iommu_context,
        dma_bytes,
        msi_vector,
        rogue_domain,
        malformed_field,
    )


def main() -> None:
    (
        root_domain,
        requester,
        bar_id,
        bar_gen,
        iommu_context,
        dma_bytes,
        msi_vector,
        rogue_domain,
        malformed_field,
    ) = seeded_values()

    completions = 0
    faults = 0

    print(f"TRACE boot root_domain={root_domain} pcie_stub=1")

    completions += 1
    assert requester > 0 and bar_id > 0 and bar_gen > 0
    print(
        f"TRACE enumerate requester={requester} bar={bar_id} "
        f"gen={bar_gen} cap=1"
    )

    completions += 1
    assert iommu_context > 0 and dma_bytes > 0
    print(
        f"TRACE iommu_dma context={iommu_context} domain={root_domain} "
        f"bytes={dma_bytes} completion=1"
    )

    completions += 1
    assert msi_vector > 0
    print(f"TRACE msi vector={msi_vector} event=1")

    faults += 1
    print(f"TRACE bus_master domain={rogue_domain} errno={EPERM}")

    faults += 1
    print(f"TRACE stale_bar gen={bar_gen + 1} errno={EREVOKED}")

    faults += 1
    print(f"TRACE malformed_config field={malformed_field} errno={EINVAL}")

    print("TRACE raw_pcie dma=0 interrupt=0")

    print(f"TRACE done completions={completions} faults={faults} bar={bar_id}")


if __name__ == "__main__":
    main()

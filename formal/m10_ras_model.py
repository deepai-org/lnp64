#!/usr/bin/env python3
"""Executable LNP64 M10 RAS/observability/assurance model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
measurement/telemetry ids, correction count, watchdog reset id, telemetry
counter count, trace-ring writes, quote id, and audit label.
"""

import os

EIO = 5
EPERM = 1


def seeded_values() -> tuple[int, int, int, int, int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 1, 1, 1, 1, 1, 3, 3, 4, 1, 7
    root_domain = (seed & 0xF) + 1
    measurement = ((seed >> 4) & 0xF) + 1
    telemetry_fdr = ((seed >> 8) & 0xF) + 1
    ecc_corrections = ((seed >> 12) & 0xF) + 1
    parity_fault = 1
    reset_id = ((seed >> 16) & 0xF) + 1
    degraded = 1
    counters_visible = ((seed >> 20) & 0xF) + 1
    trace_capacity = ((seed >> 24) & 0x7) + 1
    trace_writes = trace_capacity + ((seed >> 27) & 0x7) + 1
    quote_id = ((seed >> 16) & 0xF) + 1
    audit_label = ((seed ^ (seed >> 8)) & 0xF) + 1
    return (
        root_domain,
        measurement,
        telemetry_fdr,
        ecc_corrections,
        parity_fault,
        reset_id,
        degraded,
        counters_visible,
        trace_capacity,
        trace_writes,
        quote_id,
        audit_label,
    )


def main() -> None:
    (
        root_domain,
        measurement,
        telemetry_fdr,
        ecc_corrections,
        parity_fault,
        reset_id,
        degraded,
        counters_visible,
        trace_capacity,
        trace_writes,
        quote_id,
        audit_label,
    ) = seeded_values()
    fault_count = 0
    telemetry_reads = 0
    audit_records = 0

    print(
        f"TRACE boot root_domain={root_domain} measurement={measurement} "
        f"telemetry_fdr={telemetry_fdr}"
    )

    assert ecc_corrections >= 1
    print(f"TRACE ecc_corrected metadata=fdr_table corrections={ecc_corrections}")

    fault_count += 1
    print(f"TRACE parity_poison errno={EIO} fault={parity_fault}")

    fault_count += 1
    print(f"TRACE watchdog_timeout reset={reset_id} degraded={degraded}")

    telemetry_reads += 1
    redacted = 1
    assert counters_visible >= 1 and redacted == 1
    print(f"TRACE telemetry_read scope=aggregate counters={counters_visible} redacted=1")

    overflow = int(trace_writes > trace_capacity)
    assert overflow == 1
    print(f"TRACE trace_ring write={trace_writes} overflow=1")

    measurement_bound = measurement
    development_quote = 1
    assert measurement_bound and development_quote
    print(f"TRACE quote_stub quote={quote_id} measurement={measurement_bound} dev=1")

    audit_records += 1
    print(f"TRACE audit_mls label={audit_label} debug=denied errno={EPERM}")

    print(
        f"TRACE done faults={fault_count} telemetry_reads={telemetry_reads} "
        f"audit_records={audit_records}"
    )


if __name__ == "__main__":
    main()

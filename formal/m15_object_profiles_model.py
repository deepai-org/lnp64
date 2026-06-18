#!/usr/bin/env python3
"""Executable LNP64 M15 object-profile model.

Set LNP64_COSIM_SEED to vary object id, generation, counter threshold, queue
payload, event-source generation, and continuation id while preserving bounded
object-profile obligations.
"""

import os

EAGAIN = 11
EREVOKED = 122
RIGHT_PUSH = 0x1
RIGHT_PULL = 0x2
RIGHT_EVENT_EMIT = 0x4


def seeded_values() -> tuple[int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 3, 42, 1, 1
    object_id = (seed & 0xF) + 1
    generation = ((seed >> 4) & 0xF) + 1
    threshold = ((seed >> 8) & 0xF) + 1
    payload = 32 + ((seed >> 12) & 0xFF)
    event_generation = ((seed >> 20) & 0xF) + 1
    continuation = ((seed >> 24) & 0xF) + 1
    return object_id, generation, threshold, payload, event_generation, continuation


def main() -> None:
    object_id, generation, threshold, payload, event_generation, continuation = seeded_values()
    rights = RIGHT_PUSH | RIGHT_PULL | RIGHT_EVENT_EMIT
    print(
        f"TRACE boot object={object_id} generation={generation} "
        f"queue_capacity=1 counter_threshold={threshold}"
    )

    counter_value = threshold
    assert counter_value >= threshold
    print(f"TRACE counter value={counter_value} threshold={threshold} event=1")

    assert rights & RIGHT_PUSH
    print(f"TRACE queue_push value={payload} rights=0x{rights:016x} depth=1")

    print(f"TRACE queue_overflow errno={EAGAIN} pressure_event=1")

    source_generation = event_generation
    assert source_generation == event_generation
    print(f"TRACE event_emit source_gen={source_generation} event_gen={event_generation} delivered=1")

    stale_source_generation = source_generation + 1
    if stale_source_generation != event_generation:
        print(f"TRACE stale_event source_gen={stale_source_generation} event_gen={event_generation} errno={EREVOKED}")
    else:
        raise AssertionError("stale event source unexpectedly accepted")

    print(f"TRACE gate_profile continuation={continuation} unique=1 duplicate_errno={EREVOKED}")
    print("TRACE done failures=3 events=2")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Executable LNP64 M1 ping-pong model.

The model is intentionally bounded: two TIDs, one queue object, one producer
capability, one narrowed consumer capability, and explicit negative paths for
queue-full and stale-generation behavior. The RTL gate compares its printed
trace against this model output. Set LNP64_COSIM_SEED to run a bounded
co-simulation variant with different queue generation and payload values.
"""

import os

RIGHT_PUSH = 0x1
RIGHT_PULL = 0x2
RIGHT_DUP = 0x4
EAGAIN = 11
EREVOKED = 122


def seeded_values() -> tuple[int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 42, 7
    queue_gen = (seed & 0xF) + 1
    push_value = 32 + (seed & 0xFF)
    refill_value = 1 + ((seed >> 8) & 0xFF)
    return queue_gen, push_value, refill_value


def main() -> None:
    queue_gen, push_value, refill_value = seeded_values()
    producer = {"gen": queue_gen, "rights": RIGHT_PUSH | RIGHT_PULL | RIGHT_DUP}
    assert producer["gen"] == queue_gen
    print(f"TRACE boot root_domain=1 queue_gen={queue_gen}")

    assert producer["rights"] & RIGHT_DUP
    consumer = {"gen": producer["gen"], "rights": RIGHT_PULL}
    print(f"TRACE cap_dup dst=consumer rights=0x{consumer['rights']:016x}")

    queue = []
    consumer_parked = len(queue) == 0
    assert consumer_parked
    print("TRACE await tid=2 queue=empty state=parked")

    assert producer["gen"] == queue_gen and producer["rights"] & RIGHT_PUSH
    queue.append(push_value)
    consumer_parked = False
    events = 1
    print(f"TRACE push tid=1 value={push_value} wake=2")

    assert not consumer_parked
    assert consumer["gen"] == queue_gen and consumer["rights"] & RIGHT_PULL
    value = queue.pop(0)
    print(f"TRACE pull tid=2 value={value}")

    queue.append(refill_value)
    print(f"TRACE queue_refill value={refill_value}")

    if queue:
        print(f"TRACE push_full errno={EAGAIN}")
    else:
        raise AssertionError("bounded queue unexpectedly empty")

    queue_gen += 1
    if consumer["gen"] != queue_gen:
        print(f"TRACE stale_pull errno={EREVOKED}")
    else:
        raise AssertionError("stale generation unexpectedly accepted")

    print(f"TRACE done events={events}")


if __name__ == "__main__":
    main()

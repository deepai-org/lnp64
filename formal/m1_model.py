#!/usr/bin/env python3
"""Executable LNP64 M1 ping-pong model.

The model is intentionally bounded: two TIDs, one queue object, one producer
capability, one narrowed consumer capability, and explicit negative paths for
queue-full and stale-generation behavior. The RTL gate compares its printed
trace against this model output.
"""

RIGHT_PUSH = 0x1
RIGHT_PULL = 0x2
RIGHT_DUP = 0x4
EAGAIN = 11
EREVOKED = 122


def main() -> None:
    queue_gen = 1
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
    queue.append(42)
    consumer_parked = False
    events = 1
    print("TRACE push tid=1 value=42 wake=2")

    assert not consumer_parked
    assert consumer["gen"] == queue_gen and consumer["rights"] & RIGHT_PULL
    value = queue.pop(0)
    print(f"TRACE pull tid=2 value={value}")

    queue.append(7)
    print("TRACE queue_refill value=7")

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

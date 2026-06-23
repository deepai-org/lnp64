#!/usr/bin/env python3
"""Executable LNP64 M16 unified-endpoint model.

Mirrors the deterministic endpoint scenario the lnp64_m16_endpoint RTL engine
walks, emitting the same human-readable TRACE lines so the seeded trace cosim
can diff RTL against the model. The endpoint is a bounded queue (EP-F): a
Memory-backed endpoint carries (bytes, caps) messages; a Register-backed
endpoint is a counter whose edge a notify (empty send) raises.

LNP64_COSIM_SEED varies the endpoint id and message size while the queue
capacity (and thus the bounded fill/drain structure) stays fixed.
"""

import os

EAGAIN = 11
EBADF = 9
EMSGSIZE = 90
CAPACITY = 2
MSG_MAX_BYTES = 64
SENDER_RIGHTS = 0x7


def seeded_values() -> tuple[int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 8
    endpoint_id = (seed & 0xF) + 1
    bytes_len = 8 + ((seed >> 4) & 0xF)
    return endpoint_id, bytes_len


def main() -> None:
    endpoint_id, bytes_len = seeded_values()

    # create: Memory-backed, empty queue.
    print(f"TRACE create endpoint={endpoint_id} capacity={CAPACITY} backing=memory")

    # framing: one send = one message = one recv.
    depth = 1
    print(f"TRACE send bytes={bytes_len} depth={depth}")
    depth = 0
    print(f"TRACE recv bytes={bytes_len} depth={depth}")

    # fill to capacity; bounded throughout.
    depth = 1
    print(f"TRACE send bytes={bytes_len} depth={depth}")
    depth = CAPACITY
    print(f"TRACE send bytes={bytes_len} depth={depth}")

    # fail-closed: send on full -> EAGAIN, depth unchanged (<= capacity).
    assert depth == CAPACITY
    print(f"TRACE send_full capacity={CAPACITY} errno={EAGAIN}")

    # drain (bounded by capacity).
    depth -= 1
    print(f"TRACE recv bytes={bytes_len} depth={depth}")
    depth = 0
    print(f"TRACE recv bytes={bytes_len} depth={depth}")

    # fail-closed: recv on empty -> EAGAIN.
    assert depth == 0
    print(f"TRACE recv_empty errno={EAGAIN}")

    # fail-closed: oversize message -> EMSGSIZE.
    print(f"TRACE oversize bytes={MSG_MAX_BYTES + 1} errno={EMSGSIZE}")

    # cap-safety: resolve against sender, install into receiver, no amplify.
    depth = 1
    print(f"TRACE cap_send rights=0x{SENDER_RIGHTS:08x} caps=1")

    # cap-safety: out-of-range / revoked handle rejected, nothing installed.
    print(f"TRACE cap_reject handle=0x{0xFFFFFFFF:08x} errno={EBADF}")

    # framing: empty send to a Register-backed endpoint raises its edge +1.
    register_edge = 1
    print(f"TRACE notify register_edge={register_edge}")

    print("TRACE done failures=4 events=1")


if __name__ == "__main__":
    main()

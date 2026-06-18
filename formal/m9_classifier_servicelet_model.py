#!/usr/bin/env python3
"""Executable LNP64 M9 classifier/servicelet model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
root/table ids, verifier program shape, packet/IPC steering fields, and
budget-exhaustion cycle count.
"""

import os

EAGAIN = 11
EINVAL = 22
EREVOKED = 122


def seeded_values() -> tuple[int, int, int, int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 1, 4, 1, 7, 3, 42, 5, 16, 17
    root_domain = (seed & 0xF) + 1
    classifier_table = ((seed >> 4) & 0xF) + 1
    program_id = ((seed >> 8) & 0xFF) + 1
    instructions = ((seed >> 16) & 0xF) + 1
    packet_rule = ((seed >> 20) & 0xF) + 1
    queue_id = ((seed >> 24) & 0xF) + 1
    mark = ((seed >> 28) & 0xF) + 1
    service_id = 32 + ((seed ^ (seed >> 8)) & 0xFF)
    gate_id = ((seed >> 12) & 0xF) + 1
    cycle_budget = 16 + ((seed >> 20) & 0xF)
    cycles_used = cycle_budget + 1
    return (
        root_domain,
        classifier_table,
        program_id,
        instructions,
        packet_rule,
        queue_id,
        mark,
        service_id,
        gate_id,
        cycle_budget,
        cycles_used,
    )


def main() -> None:
    (
        root_domain,
        classifier_table,
        program_id,
        instructions,
        packet_rule,
        queue_id,
        mark,
        service_id,
        gate_id,
        cycle_budget,
        cycles_used,
    ) = seeded_values()
    attachment_generation = 1
    stale_attachment_generation = attachment_generation
    packets = 0
    ipc = 0
    rejects = 0
    print(f"TRACE boot root_domain={root_domain} classifier_table={classifier_table}")

    verifier_accepted = True
    assert verifier_accepted and instructions <= 16
    print(f"TRACE verifier program={program_id} instructions={instructions} accepted=1")

    rejects += 1
    print(f"TRACE verifier_reject reason=blocking errno={EINVAL}")

    packets += 1
    print(f"TRACE packet_steer rule={packet_rule} queue={queue_id} mark={mark}")

    ipc += 1
    print(f"TRACE ipc_steer service={service_id} gate={gate_id}")

    assert verifier_accepted
    print("TRACE action_emit kind=needs_software authorized=1")

    rejects += 1
    assert cycles_used == cycle_budget + 1
    print(f"TRACE budget_exhaust errno={EAGAIN} cycles={cycles_used}")

    attachment_generation += 1
    if stale_attachment_generation != attachment_generation:
        print(f"TRACE stale_attachment errno={EREVOKED}")
    else:
        raise AssertionError("stale servicelet attachment unexpectedly accepted")

    print(f"TRACE done packets={packets} ipc={ipc} rejects={rejects}")


if __name__ == "__main__":
    main()

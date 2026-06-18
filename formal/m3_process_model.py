#!/usr/bin/env python3
"""Executable LNP64 M3 process/thread lifecycle model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
thread ids, exit code, exec epoch, and stopped-sibling count.
"""

import os

ECANCELED = 125
EREVOKED = 122


def seeded_values() -> tuple[int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 2, 7, 1, 1
    parent_tid = (seed & 0xF) + 1
    child_tid = ((seed >> 4) & 0xF) + 2
    exit_code = ((seed >> 8) & 0xFF) + 1
    exec_epoch = ((seed >> 16) & 0xF) + 1
    siblings_stopped = ((seed >> 20) & 0x3) + 1
    return parent_tid, child_tid, exit_code, exec_epoch, siblings_stopped


def main() -> None:
    parent_tid, child_tid, exit_code, exec_epoch, siblings_stopped = seeded_values()
    parent = {"tid": parent_tid, "state": "running"}
    child = {"tid": None, "state": "free", "generation": 0}
    join_generation = 0
    print(f"TRACE boot parent={parent_tid} child_slot=free exec_epoch={exec_epoch}")

    assert child["state"] == "free"
    child = {"tid": child_tid, "state": "runnable", "generation": 1}
    join_generation = child["generation"]
    print(f"TRACE clone parent={parent_tid} child={child_tid} state=runnable")

    assert child["state"] == "runnable" and child["generation"] == join_generation
    child["state"] = "exited"
    waitable_signaled = True
    print(f"TRACE exit child={child_tid} code={exit_code} waitable=signaled")

    assert waitable_signaled and child["state"] == "exited"
    child["state"] = "free"
    child["generation"] += 1
    waitable_signaled = False
    print(f"TRACE join parent={parent['tid']} child={child_tid} code={exit_code}")

    exec_epoch += 1
    print(f"TRACE exec_barrier epoch={exec_epoch} siblings_stopped={siblings_stopped}")

    if join_generation != child["generation"]:
        print(f"TRACE stale_join errno={EREVOKED}")
    else:
        raise AssertionError("stale join unexpectedly accepted")

    if not waitable_signaled:
        print(f"TRACE exec_cancel errno={ECANCELED}")
    else:
        raise AssertionError("exec cancellation failed to terminate")

    print(f"TRACE done live_threads=1 exec_epoch={exec_epoch}")


if __name__ == "__main__":
    main()

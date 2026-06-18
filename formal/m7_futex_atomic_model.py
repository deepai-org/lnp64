#!/usr/bin/env python3
"""Executable LNP64 M7 futex/atomic model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
root domain, atomic values, futex address, and bucket id.
"""

import os

EAGAIN = 11
EREVOKED = 122
FUTEX_ADDR = 4096


def seeded_values() -> tuple[int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 0, 1, 0, 2, FUTEX_ADDR, 1, 3
    root_domain = (seed & 0xF) + 1
    initial_atomic = (seed >> 4) & 0xFF
    success_desired = initial_atomic + ((seed >> 12) & 0xF) + 1
    fail_expected = success_desired + ((seed >> 16) & 0xF) + 1
    fail_desired = success_desired + ((seed >> 20) & 0xF) + 2
    futex_addr = FUTEX_ADDR + (((seed >> 24) & 0xF) << 3)
    bucket_id = ((seed >> 28) & 0xF) + 1
    timer_deadline = ((seed >> 4) & 0xF) + 3
    return root_domain, initial_atomic, success_desired, fail_expected, fail_desired, futex_addr, bucket_id, timer_deadline


def main() -> None:
    (
        root_domain,
        atomic_word,
        success_desired,
        fail_expected,
        fail_desired,
        futex_addr,
        bucket_id,
        timer_deadline,
    ) = seeded_values()
    success_expected = atomic_word
    address_generation = 1
    stale_generation = address_generation
    atomics = 0
    wakes = 0
    waiter_parked = False
    print(f"TRACE boot root_domain={root_domain} atomic_word={atomic_word}")

    old = atomic_word
    if old == success_expected:
        atomic_word = success_desired
        atomics += 1
        print(f"TRACE cmpxchg expected={success_expected} desired={success_desired} old={old} result=ok")
    else:
        raise AssertionError("first compare-exchange unexpectedly failed")

    old = atomic_word
    if old != fail_expected:
        atomics += 1
        print(f"TRACE cmpxchg expected={fail_expected} desired={fail_desired} old={old} errno={EAGAIN}")
    else:
        raise AssertionError("second compare-exchange unexpectedly succeeded")

    if atomic_word == success_desired:
        waiter_parked = True
        print(f"TRACE futex_wait addr={futex_addr} expected={success_desired} state=parked")
    else:
        raise AssertionError("futex wait expected value mismatch")

    if waiter_parked:
        waiter_parked = False
        wakes += 1
        print(f"TRACE futex_wake addr={futex_addr} woken=1")
    else:
        raise AssertionError("lost futex waiter before wake")

    waiter_parked = True
    print(f"TRACE timer_wait deadline={timer_deadline} state=parked")
    if waiter_parked:
        waiter_parked = False
        wakes += 1
        print(f"TRACE timer_expire deadline={timer_deadline} woken=1")
    else:
        raise AssertionError("lost timer waiter before expiry")

    print(f"TRACE bucket_spill bucket={bucket_id} preserved=1")

    address_generation += 1
    if stale_generation != address_generation:
        print(f"TRACE stale_futex errno={EREVOKED}")
    else:
        raise AssertionError("stale futex address unexpectedly accepted")

    assert wakes == 0 or not waiter_parked
    print(f"TRACE done wakes={wakes} atomics={atomics}")


if __name__ == "__main__":
    main()

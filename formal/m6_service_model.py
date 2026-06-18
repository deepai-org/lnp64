#!/usr/bin/env python3
"""Executable LNP64 M6 typed-control/namespace/service-boundary model.

Set LNP64_COSIM_SEED to run a bounded co-simulation variant with different
namespace/service ids, path length, continuation ids, returned object id, and
returned rights.
"""

import os

ECANCELED = 125
EIO = 5
EREVOKED = 122
RIGHT_READ = 0x1
RIGHT_WRITE = 0x2


def seeded_values() -> tuple[int, int, int, int, int, int, int, int, int]:
    seed = int(os.environ.get("LNP64_COSIM_SEED", "0"), 0)
    if seed == 0:
        return 1, 1, 8, 1, 1, 1, 9, RIGHT_READ, 2
    root_domain = (seed & 0xF) + 1
    namespace_root = ((seed >> 4) & 0xF) + 1
    path_len = ((seed >> 8) & 0x3F) + 1
    service_id = ((seed >> 14) & 0x3F) + 1
    op_id = ((seed >> 20) & 0xF) + 1
    continuation = ((seed >> 24) & 0xF) + 1
    returned_rights = RIGHT_WRITE if ((seed >> 28) & 0x1) else RIGHT_READ
    cap_object = service_id + ((seed >> 29) & 0x7) + 8
    cancel_continuation = continuation + 1
    return (
        root_domain,
        namespace_root,
        path_len,
        service_id,
        op_id,
        continuation,
        cap_object,
        returned_rights,
        cancel_continuation,
    )


def main() -> None:
    (
        root_domain,
        namespace_root,
        path_len,
        service_id,
        op_id,
        continuation,
        cap_object,
        returned_rights,
        cancel_continuation,
    ) = seeded_values()
    service_generation = 1
    stale_service_generation = service_generation
    installed_caps = 0
    completions = 0
    print(f"TRACE boot root_domain={root_domain} namespace_root={namespace_root}")

    envelope = {"op": "open_at", "version": 1, "profile": "namespace", "valid": True}
    assert envelope["valid"] and envelope["version"] == 1
    print("TRACE envelope op=open_at version=1 profile=namespace valid=1")

    selector = 3
    assert selector == 3 and path_len <= 64
    print(f"TRACE ns_dispatch selector={selector} path_len={path_len} service={service_id}")

    print(f"TRACE service_request op_id={op_id} continuation={continuation} state=pending")

    requested_rights = RIGHT_READ | RIGHT_WRITE
    assert returned_rights & ~requested_rights == 0
    assert returned_rights != requested_rights
    installed_caps += 1
    print(f"TRACE cap_proposal object={cap_object} rights=0x{returned_rights:016x} installed=1")

    completions += 1
    print(f"TRACE service_cancel continuation={cancel_continuation} errno={ECANCELED}")

    service_generation += 1
    if stale_service_generation != service_generation:
        print(f"TRACE stale_service errno={EREVOKED}")
    else:
        raise AssertionError("stale service generation unexpectedly accepted")

    completions += 1
    print(f"TRACE crash_completion errno={EIO}")
    print(f"TRACE done installed_caps={installed_caps} completions={completions}")


if __name__ == "__main__":
    main()

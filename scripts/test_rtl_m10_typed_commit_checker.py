#!/usr/bin/env python3
"""Self-test the M10 typed RAS commit checker failure modes."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m10_typed_commit_trace.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m10_typed_commit_trace", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load M10 checker module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def expect_failure(expected: str, action) -> None:
    try:
        action()
    except SystemExit as exc:
        require(exc.code != 0, "checker failure unexpectedly used success exit code")
        require(expected in str(exc), f"checker failure did not include {expected!r}: {exc}")
    else:
        raise SystemExit(f"expected checker failure for: {expected}")


def encode_bits(record: dict, fields: tuple[str, ...], widths: tuple[int, ...]) -> str:
    value = 0
    for field, width in zip(fields, widths, strict=True):
        raw = record[field]
        require(isinstance(raw, int) and 0 <= raw < (1 << width), f"{field}={raw} does not fit in {width} bits")
        value = (value << width) | raw
    return f"{value:0{(sum(widths) + 3) // 4}x}"


def build_valid(checker):
    cf, cw, sf, sw, ops = checker.load_schema()

    def commit(op, status, fc, tr, ar):
        return {"record": checker.COMMIT_NAME, "op": op, "status": status, "root_domain": 1,
                "fault_count": fc, "telemetry_reads": tr, "audit_records": ar, "quote_id": 1, "reset_id": 1}

    def state(op, status, fc, tr, ar, **flags):
        base = {f: 0 for f in sf}
        base["record"] = checker.STATE_NAME
        base.update({"op": op, "status": status, "fault_count": fc, "telemetry_reads": tr,
                     "audit_records": ar, "trace_capacity": 3})
        base.update(flags)
        return base

    o = ops
    f = dict(boot_measured=1, telemetry_fdr_present=1)
    commits = [
        commit(o["LNP64_M10_COMMIT_BOOT_MEASURE"], checker.ERR_OK, 0, 0, 0),
        commit(o["LNP64_M10_COMMIT_ECC_CORRECT"], checker.ERR_OK, 0, 0, 0),
        commit(o["LNP64_M10_COMMIT_PARITY_POISON"], checker.ERR_EIO, 0, 0, 0),
        commit(o["LNP64_M10_COMMIT_WATCHDOG"], checker.ERR_OK, 1, 0, 0),
        commit(o["LNP64_M10_COMMIT_TELEMETRY_READ"], checker.ERR_OK, 2, 0, 0),
        commit(o["LNP64_M10_COMMIT_TRACE_RING"], checker.ERR_OK, 2, 1, 0),
        commit(o["LNP64_M10_COMMIT_QUOTE"], checker.ERR_OK, 2, 1, 0),
        commit(o["LNP64_M10_COMMIT_AUDIT_MLS"], checker.ERR_EPERM, 2, 1, 0),
    ]
    states = [
        state(o["LNP64_M10_COMMIT_BOOT_MEASURE"], checker.ERR_OK, 0, 0, 0, **f),
        state(o["LNP64_M10_COMMIT_ECC_CORRECT"], checker.ERR_OK, 0, 0, 0, **f, ecc_corrected=1),
        state(o["LNP64_M10_COMMIT_PARITY_POISON"], checker.ERR_EIO, 1, 0, 0, **f, ecc_corrected=1, parity_poison_faulted=1),
        state(o["LNP64_M10_COMMIT_WATCHDOG"], checker.ERR_OK, 2, 0, 0, **f, ecc_corrected=1, parity_poison_faulted=1, watchdog_timed_out=1, local_reset_seen=1, degraded_state=1),
        state(o["LNP64_M10_COMMIT_TELEMETRY_READ"], checker.ERR_OK, 2, 1, 0, **f, ecc_corrected=1, parity_poison_faulted=1, watchdog_timed_out=1, local_reset_seen=1, degraded_state=1, telemetry_scoped=1, telemetry_redacted=1),
        state(o["LNP64_M10_COMMIT_TRACE_RING"], checker.ERR_OK, 2, 1, 0, **f, ecc_corrected=1, parity_poison_faulted=1, watchdog_timed_out=1, local_reset_seen=1, degraded_state=1, telemetry_scoped=1, telemetry_redacted=1, trace_overflowed=1, trace_writes=4),
        state(o["LNP64_M10_COMMIT_QUOTE"], checker.ERR_OK, 2, 1, 0, **f, ecc_corrected=1, parity_poison_faulted=1, watchdog_timed_out=1, local_reset_seen=1, degraded_state=1, telemetry_scoped=1, telemetry_redacted=1, trace_overflowed=1, trace_writes=4, quote_measurement_bound=1, quote_development_marked=1),
        state(o["LNP64_M10_COMMIT_AUDIT_MLS"], checker.ERR_EPERM, 2, 1, 1, **f, ecc_corrected=1, parity_poison_faulted=1, watchdog_timed_out=1, local_reset_seen=1, degraded_state=1, telemetry_scoped=1, telemetry_redacted=1, trace_overflowed=1, trace_writes=4, quote_measurement_bound=1, quote_development_marked=1, audit_recorded=1, mls_denied=1, debug_denied=1, counts_exact=1),
    ]
    return commits, states, ops, cf, cw, sf, sw


def main() -> None:
    checker = load_checker()
    commits, states, ops, cf, cw, sf, sw = build_valid(checker)

    commit_bits = [encode_bits(c, cf, cw) for c in commits]
    state_bits = [encode_bits(s, sf, sw) for s in states]
    checker.check_bits(commits, commit_bits, cf, cw, "M10 typed commit")
    checker.check_bits(states, state_bits, sf, sw, "M10 state projection")
    checker.check_transition_trace(commits, states, ops)

    scrambled = copy.deepcopy(commits)
    scrambled[0], scrambled[1] = scrambled[1], scrambled[0]
    expect_failure("sequence drifted", lambda: checker.check_transition_trace(scrambled, states, ops))

    # Telemetry not redacted (evidence honesty).
    bad_redact = copy.deepcopy(states)
    bad_redact[4]["telemetry_redacted"] = 0
    expect_failure("was not redacted", lambda: checker.check_transition_trace(commits, bad_redact, ops))

    # Audit MLS not denying cross-label access.
    bad_mls = copy.deepcopy(states)
    bad_mls[7]["mls_denied"] = 0
    expect_failure("did not deny cross-label/debug access", lambda: checker.check_transition_trace(commits, bad_mls, ops))

    bad_bits = list(commit_bits)
    bad_bits[0] = encode_bits({**commits[0], "quote_id": 9}, cf, cw)
    expect_failure("packed decode drift", lambda: checker.check_bits(commits, bad_bits, cf, cw, "M10 typed commit"))

    print("rtl m10 typed commit checker self-test ok")


if __name__ == "__main__":
    main()

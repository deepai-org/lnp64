#!/usr/bin/env python3
"""Self-test the offline M7 scheduler refinement witness checker.

Builds a synthetic but schema-valid M7 witness from the shared M7 transition
model, confirms the consumer accepts it, then mutates it in ways that must fail
closed: a broken records hash, a drifted schema width, a packed bit vector that
disagrees with its projection, and a scrambled commit op sequence.
"""

from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m7_typed_commit_trace.py"
CONSUMER = ROOT / "scripts/check_rtl_m7_witness.py"
M7_TEST = ROOT / "scripts/test_rtl_m7_typed_commit_checker.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    require(spec is not None and spec.loader is not None, f"could not load {name}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def run_consumer(consumer, artifact: dict) -> None:
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
        # Mirror the producer, which serializes with sort_keys=True (so loaded
        # dict key order is alphabetical, not schema order).
        json.dump(artifact, handle, sort_keys=True)
        path = handle.name
    saved_argv = sys.argv
    sys.argv = ["check_rtl_m7_witness", path]
    try:
        consumer.main()
    finally:
        sys.argv = saved_argv
        Path(path).unlink(missing_ok=True)


def expect_failure(consumer, artifact: dict, expected: str) -> None:
    try:
        run_consumer(consumer, artifact)
    except SystemExit as exc:
        require(exc.code != 0, "consumer failure unexpectedly used success exit code")
        require(expected in str(exc), f"consumer failure did not include {expected!r}: {exc}")
    else:
        raise SystemExit(f"expected consumer failure for: {expected}")


def build_artifact(checker, m7_test):
    commit_fields, commit_widths, state_fields, state_widths, ops = checker.load_schema()
    commits, states = m7_test.build_valid_run(checker, ops)
    commit_bits = [m7_test.encode_bits(c, commit_fields, commit_widths) for c in commits]
    state_bits = [m7_test.encode_bits(s, state_fields, state_widths) for s in states]
    records = []
    for index, (commit, cbits, state, sbits) in enumerate(zip(commits, commit_bits, states, state_bits, strict=True)):
        records.append(
            {
                "index": index,
                "op": commit["op"],
                "status": commit["status"],
                "commit": {field: commit[field] for field in commit_fields},
                "commit_bits": cbits,
                "state": {field: state[field] for field in state_fields},
                "state_bits": sbits,
            }
        )
    artifact = {
        "schema": checker.WITNESS_SCHEMA,
        "commit_schema": {"fields": list(commit_fields), "widths": list(commit_widths), "width": sum(commit_widths)},
        "state_schema": {"fields": list(state_fields), "widths": list(state_widths), "width": sum(state_widths)},
        "ops": {
            "cmpxchg_success": ops.cmpxchg_success,
            "cmpxchg_fail": ops.cmpxchg_fail,
            "futex_wait": ops.futex_wait,
            "futex_wake": ops.futex_wake,
            "timer_wait": ops.timer_wait,
            "timer_expire": ops.timer_expire,
            "consume_wake": ops.consume_wake,
            "reject_stale_address": ops.reject_stale_address,
        },
        "record_count": len(records),
        "records": records,
    }
    artifact["records_sha256"] = checker.sha256_json(records)
    return artifact, checker, commit_fields, commit_widths


def main() -> None:
    checker = load_module("check_rtl_m7_typed_commit_trace", CHECKER)
    m7_test = load_module("test_rtl_m7_typed_commit_checker", M7_TEST)
    consumer = load_module("check_rtl_m7_witness", CONSUMER)

    artifact, checker, commit_fields, commit_widths = build_artifact(checker, m7_test)

    # Positive: synthetic witness must pass.
    run_consumer(consumer, artifact)

    bad_hash = copy.deepcopy(artifact)
    bad_hash["records_sha256"] = "0" * 64
    expect_failure(consumer, bad_hash, "records hash does not recompute")

    bad_width = copy.deepcopy(artifact)
    bad_width["commit_schema"]["widths"][0] += 1
    expect_failure(consumer, bad_width, "commit schema widths drifted")

    bad_bits = copy.deepcopy(artifact)
    mutated = dict(bad_bits["records"][0]["commit"])
    mutated["tid"] = mutated["tid"] + 1
    bad_bits["records"][0]["commit_bits"] = m7_test.encode_bits(mutated, commit_fields, commit_widths)
    bad_bits["records_sha256"] = checker.sha256_json(bad_bits["records"])
    expect_failure(consumer, bad_bits, "packed decode drift")

    scrambled = copy.deepcopy(artifact)
    scrambled["records"][0], scrambled["records"][1] = scrambled["records"][1], scrambled["records"][0]
    scrambled["records"][0]["index"] = 0
    scrambled["records"][1]["index"] = 1
    scrambled["records_sha256"] = checker.sha256_json(scrambled["records"])
    expect_failure(consumer, scrambled, "sequence drifted")

    print("rtl m7 witness checker self-test ok")


if __name__ == "__main__":
    main()

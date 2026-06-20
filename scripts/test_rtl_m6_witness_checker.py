#!/usr/bin/env python3
"""Self-test the offline M6 VMA/MMU refinement witness checker."""

from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m6_typed_commit_trace.py"
CONSUMER = ROOT / "scripts/check_rtl_m6_witness.py"
M6_TEST = ROOT / "scripts/test_rtl_m6_typed_commit_checker.py"


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
        json.dump(artifact, handle, sort_keys=True)
        path = handle.name
    saved_argv = sys.argv
    sys.argv = ["check_rtl_m6_witness", path]
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


def main() -> None:
    checker = load_module("check_rtl_m6_typed_commit_trace", CHECKER)
    m6_test = load_module("test_rtl_m6_typed_commit_checker", M6_TEST)
    consumer = load_module("check_rtl_m6_witness", CONSUMER)

    commits, states, ops, cf, cw, sf, sw = m6_test.build_valid(checker)
    commit_bits = [m6_test.encode_bits(c, cf, cw) for c in commits]
    state_bits = [m6_test.encode_bits(s, sf, sw) for s in states]
    artifact = checker.build_witness(commits, commit_bits, states, state_bits, cf, cw, sf, sw)

    # Positive.
    run_consumer(consumer, artifact)

    bad_hash = copy.deepcopy(artifact)
    bad_hash["records_sha256"] = "0" * 64
    expect_failure(consumer, bad_hash, "records hash does not recompute")

    bad_width = copy.deepcopy(artifact)
    bad_width["commit_schema"]["widths"][0] += 1
    expect_failure(consumer, bad_width, "commit schema widths drifted")

    bad_bits = copy.deepcopy(artifact)
    mutated = dict(bad_bits["records"][0]["commit"])
    mutated["service_id"] = mutated["service_id"] + 1
    bad_bits["records"][0]["commit_bits"] = m6_test.encode_bits(mutated, cf, cw)
    bad_bits["records_sha256"] = checker.sha256_json(bad_bits["records"])
    expect_failure(consumer, bad_bits, "packed decode drift")

    # No authority amplification: returned rights cannot exceed requested.
    amplify = copy.deepcopy(artifact)
    amplify["records"][3]["commit"]["returned_rights"] = 0xF
    amplify["records"][3]["commit_bits"] = m6_test.encode_bits(amplify["records"][3]["commit"], cf, cw)
    amplify["records_sha256"] = checker.sha256_json(amplify["records"])
    expect_failure(consumer, amplify, "returned amplified rights")

    print("rtl m6 witness checker self-test ok")


if __name__ == "__main__":
    main()

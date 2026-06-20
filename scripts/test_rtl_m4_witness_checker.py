#!/usr/bin/env python3
"""Self-test the offline M4 VMA/MMU refinement witness checker."""

from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_rtl_m4_typed_commit_trace.py"
CONSUMER = ROOT / "scripts/check_rtl_m4_witness.py"
M4_TEST = ROOT / "scripts/test_rtl_m4_typed_commit_checker.py"


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
    sys.argv = ["check_rtl_m4_witness", path]
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
    checker = load_module("check_rtl_m4_typed_commit_trace", CHECKER)
    m4_test = load_module("test_rtl_m4_typed_commit_checker", M4_TEST)
    consumer = load_module("check_rtl_m4_witness", CONSUMER)

    commits, states, ops, cf, cw, sf, sw = m4_test.build_valid(checker)
    commit_bits = [m4_test.encode_bits(c, cf, cw) for c in commits]
    state_bits = [m4_test.encode_bits(s, sf, sw) for s in states]
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
    mutated["vma_id"] = mutated["vma_id"] + 1
    bad_bits["records"][0]["commit_bits"] = m4_test.encode_bits(mutated, cf, cw)
    bad_bits["records_sha256"] = checker.sha256_json(bad_bits["records"])
    expect_failure(consumer, bad_bits, "packed decode drift")

    bad_wx = copy.deepcopy(artifact)
    bad_wx["records"][0]["state"]["wx_enforced"] = 0
    bad_wx["records"][0]["state_bits"] = m4_test.encode_bits(bad_wx["records"][0]["state"], sf, sw)
    bad_wx["records_sha256"] = checker.sha256_json(bad_wx["records"])
    expect_failure(consumer, bad_wx, "W^X invariant")

    print("rtl m4 witness checker self-test ok")


if __name__ == "__main__":
    main()

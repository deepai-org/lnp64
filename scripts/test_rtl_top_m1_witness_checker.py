#!/usr/bin/env python3
"""Self-test the offline lnp64_top M1 refinement witness checker.

Builds a synthetic but schema-valid witness artifact, confirms the checker
accepts it, then mutates it in ways that must fail closed: a broken records
hash, a drifted schema width, a packed bit vector that disagrees with its
projection, an op/status mismatch, and an authority-amplifying OK transition.
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
CHECKER = ROOT / "scripts/check_rtl_top_m1_witness.py"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def load_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_top_m1_witness", CHECKER)
    require(spec is not None and spec.loader is not None, "could not load witness checker module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def encode_bits(record: dict, fields: tuple[str, ...], widths: tuple[int, ...]) -> str:
    value = 0
    for field, width in zip(fields, widths, strict=True):
        raw = record[field]
        require(isinstance(raw, int), f"{field} must be an integer")
        require(0 <= raw < (1 << width), f"{field}={raw} does not fit in {width} bits")
        value = (value << width) | raw
    hex_digits = (sum(widths) + 3) // 4
    return f"{value:0{hex_digits}x}"


def run_checker(checker, artifact: dict) -> None:
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
        json.dump(artifact, handle)
        path = handle.name
    saved_argv = sys.argv
    sys.argv = ["check_rtl_top_m1_witness", path]
    try:
        checker.main()
    finally:
        sys.argv = saved_argv
        Path(path).unlink(missing_ok=True)


def expect_failure(checker, artifact: dict, expected: str) -> None:
    try:
        run_checker(checker, artifact)
    except SystemExit as exc:
        require(exc.code != 0, "checker failure unexpectedly used success exit code")
        require(expected in str(exc), f"checker failure did not include {expected!r}: {exc}")
    else:
        raise SystemExit(f"expected checker failure for: {expected}")


def build_record(
    index: int,
    op: int,
    status: int,
    commit_fields: tuple[str, ...],
    commit_widths: tuple[int, ...],
    state_fields: tuple[str, ...],
    state_widths: tuple[int, ...],
    state_overrides: dict[str, int],
) -> dict:
    commit = {field: 0 for field in commit_fields}
    commit["op"] = op
    commit["status"] = status
    pre = {field: 0 for field in state_fields}
    pre["op"] = op
    pre["status"] = status
    post = {field: 0 for field in state_fields}
    post["op"] = op
    post["status"] = status
    for field, value in state_overrides.items():
        post[field] = value
    return {
        "index": index,
        "pc": 0,
        "tile_id": 0,
        "op": op,
        "status": status,
        "commit": commit,
        "commit_bits": encode_bits(commit, commit_fields, commit_widths),
        "pre_state": pre,
        "pre_state_bits": encode_bits(pre, state_fields, state_widths),
        "post_state": post,
        "post_state_bits": encode_bits(post, state_fields, state_widths),
    }


def build_artifact(checker) -> dict:
    schema = checker.load_schema()
    commit_fields, commit_widths = checker.load_record_schema(schema, checker.COMMIT_RECORD)
    state_fields, state_widths = checker.load_record_schema(schema, checker.STATE_RECORD)
    commit_ops = checker.load_commit_ops(schema)
    # A stale-rejection non-OK transition: authority projection unchanged, post
    # marks stale_rejected. status 116 == EREVOKED/stale path in the mirror.
    record = build_record(
        0,
        commit_ops["RejectStale"],
        116,
        commit_fields,
        commit_widths,
        state_fields,
        state_widths,
        {"stale_rejected": 1},
    )
    records = [record]
    artifact = {
        "schema": checker.WITNESS_SCHEMA,
        "source_log": "synthetic.log",
        "commit_schema": {
            "fields": list(commit_fields),
            "widths": list(commit_widths),
            "width": sum(commit_widths),
        },
        "state_schema": {
            "fields": list(state_fields),
            "widths": list(state_widths),
            "width": sum(state_widths),
        },
        "commit_count": len(records),
        "records": records,
    }
    artifact["records_sha256"] = checker.sha256_json(records)
    return artifact, commit_fields, commit_widths


def main() -> None:
    checker = load_checker()
    artifact, commit_fields, commit_widths = build_artifact(checker)

    # Positive: the synthetic witness must pass.
    run_checker(checker, artifact)

    # Broken integrity hash.
    bad_hash = copy.deepcopy(artifact)
    bad_hash["records_sha256"] = "0" * 64
    expect_failure(checker, bad_hash, "records hash does not recompute")

    # Schema width drift.
    bad_width = copy.deepcopy(artifact)
    bad_width["commit_schema"]["widths"][0] += 1
    expect_failure(checker, bad_width, "commit schema widths drifted")

    # Packed commit bits disagree with the stored projection.
    bad_bits = copy.deepcopy(artifact)
    mutated = dict(bad_bits["records"][0]["commit"])
    mutated["object_id"] = 7
    bad_bits["records"][0]["commit_bits"] = encode_bits(mutated, commit_fields, commit_widths)
    bad_bits["records_sha256"] = checker.sha256_json(bad_bits["records"])
    expect_failure(checker, bad_bits, "packed bits drifted from projection")

    # Top-level op/status drifted from the commit projection.
    bad_status = copy.deepcopy(artifact)
    bad_status["records"][0]["status"] = 117
    bad_status["records_sha256"] = checker.sha256_json(bad_status["records"])
    expect_failure(checker, bad_status, "op/status drifted")

    # Authority-amplifying OK transition: an accepted Push without PUSH right.
    schema = checker.load_schema()
    state_fields, state_widths = checker.load_record_schema(schema, checker.STATE_RECORD)
    commit_ops = checker.load_commit_ops(schema)
    amplify = copy.deepcopy(artifact)
    bad_push = build_record(
        0,
        commit_ops["Push"],
        0,
        commit_fields,
        commit_widths,
        state_fields,
        state_widths,
        {},
    )
    amplify["records"] = [bad_push]
    amplify["records_sha256"] = checker.sha256_json(amplify["records"])
    expect_failure(checker, amplify, "push 0 accepted without PUSH right")

    print("rtl top-level M1 witness checker self-test ok")


if __name__ == "__main__":
    main()

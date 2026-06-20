#!/usr/bin/env python3
"""Offline checker for the M12 storage-barrier refinement witness artifact.

scripts/check_rtl_m12_typed_commit_trace.py can emit a
`lnp64_m12_storage_refinement_witness_v1` JSON artifact (via LNP64_RTL_M12_WITNESS_OUT)
capturing every storage commit, its authority-state projection, and their
packed bit vectors. This checker re-validates that artifact without re-running
the simulator, reusing the M12 transition logic (single source -- no duplicated
relation):

1. artifact schema, commit/state schema field order/widths/total widths match
   the shared RTL schema;
2. the canonical records hash recomputes (artifact integrity);
3. every packed commit/state bit vector decodes back to its stored projection
   fields (packed-bit faithfulness); and
4. the commit/state sequence satisfies the M12 transition contract
   (check_transition_trace) -- op order, per-op authority invariants, and the
   object-binding/never-raw-authority invariants per projection.
"""

import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
M12_CHECKER = ROOT / "scripts/check_rtl_m12_typed_commit_trace.py"


def fail(message: str) -> None:
    raise SystemExit(f"M12 witness check failed: {message}")


def load_m12_checker():
    spec = importlib.util.spec_from_file_location("check_rtl_m12_typed_commit_trace", M12_CHECKER)
    if spec is None or spec.loader is None:
        fail("could not load M12 checker module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def require_schema_block(label: str, block: object, fields: tuple, widths: tuple) -> None:
    if not isinstance(block, dict):
        fail(f"{label} schema block missing")
    if tuple(block.get("fields", ())) != tuple(fields):
        fail(f"{label} schema field order drifted from shared schema")
    if tuple(block.get("widths", ())) != tuple(widths):
        fail(f"{label} schema widths drifted from shared schema")
    if block.get("width") != sum(widths):
        fail(f"{label} schema total width drifted from shared schema")


def main() -> int:
    m12 = load_m12_checker()
    witness_path = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "build/lnp64-m12-storage-refinement-witness.json"
    if not witness_path.exists():
        fail(f"missing witness artifact {witness_path}")
    artifact = json.loads(witness_path.read_text(encoding="utf-8"))

    if artifact.get("schema") != m12.WITNESS_SCHEMA:
        fail(f"unexpected witness schema {artifact.get('schema')!r}")

    commit_fields, commit_widths, state_fields, state_widths, ops = m12.load_schema()
    require_schema_block("commit", artifact.get("commit_schema"), commit_fields, commit_widths)
    require_schema_block("state", artifact.get("state_schema"), state_fields, state_widths)

    records = artifact.get("records")
    if not isinstance(records, list) or not records:
        fail("witness artifact has no records")
    if artifact.get("record_count") != len(records):
        fail(f"record_count {artifact.get('record_count')!r} does not match {len(records)} records")
    if m12.sha256_json(records) != artifact.get("records_sha256"):
        fail("records hash does not recompute; witness artifact is not internally consistent")

    commits: list[dict] = []
    states: list[dict] = []
    commit_bits: list[str] = []
    state_bits: list[str] = []
    for idx, record in enumerate(records):
        if record.get("index") != idx:
            fail(f"record {idx} has out-of-order index {record.get('index')!r}")
        commit = record.get("commit")
        state = record.get("state")
        if not isinstance(commit, dict) or not isinstance(state, dict):
            fail(f"record {idx} missing commit/state projection")
        if commit.get("op") != record.get("op") or commit.get("status") != record.get("status"):
            fail(f"record {idx} op/status drifted from commit projection")
        if set(commit.keys()) != set(commit_fields):
            fail(f"record {idx} commit fields drifted from schema")
        if set(state.keys()) != set(state_fields):
            fail(f"record {idx} state fields drifted from schema")
        cbits = record.get("commit_bits")
        sbits = record.get("state_bits")
        if not isinstance(cbits, str) or not isinstance(sbits, str):
            fail(f"record {idx} missing packed bits")
        commits.append(dict(commit))
        states.append(dict(state))
        commit_bits.append(cbits)
        state_bits.append(sbits)

    m12.check_bits(commits, commit_bits, commit_fields, commit_widths, "M12 witness commit")
    m12.check_bits(states, state_bits, state_fields, state_widths, "M12 witness state projection")
    m12.check_transition_trace(commits, states, ops)

    print(f"rtl m12 witness ok ({len(records)} records, {witness_path.name})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Run/validate the S0 runtime typed transition trace against the shared schema."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path


ROOT = Path(os.environ.get("LNP64_TYPED_TRACE_ROOT", str(Path(__file__).resolve().parents[1])))
SCHEMA = ROOT / "rtl/schema/lnp64_shared_schema.json"
S0_LOG = Path(os.environ.get("LNP64_S0_TRACE_LOG", "/tmp/lnp64_rtl_s0_sim.log"))
S0_GATE = ROOT / "scripts/run_rtl_s0.sh"


REQUIRED_RECORDS = [
    "lnp64_retire_submit_t",
    "lnp64_thread_sched_t",
    "lnp64_event_t",
    "lnp64_fault_t",
    "lnp64_tlb_cache_invalidate_t",
    "lnp64_coherence_txn_t",
    "lnp64_trace_t",
]


def fail(message: str) -> None:
    raise SystemExit(f"rtl typed trace contract check failed: {message}")


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def read_json(path: Path) -> dict:
    require(path.exists(), f"missing {path.relative_to(ROOT)}")
    return json.loads(path.read_text(encoding="utf-8"))


def run_s0_if_needed() -> str:
    use_existing = os.environ.get("LNP64_TYPED_TRACE_USE_EXISTING") == "1"
    if not use_existing:
        result = subprocess.run(
            ["bash", str(S0_GATE)],
            cwd=ROOT,
            check=False,
            text=True,
            capture_output=True,
        )
        output = (result.stdout or "") + (result.stderr or "")
        if result.returncode != 0:
            if output:
                print(output, end="")
            fail(f"{S0_GATE.relative_to(ROOT)} failed with exit code {result.returncode}")
        return output

    require(S0_LOG.exists(), f"missing existing S0 log {S0_LOG}")
    return S0_LOG.read_text(encoding="utf-8")


def schema_fields(schema: dict, record: str) -> list[str]:
    entries = schema.get("records", {}).get(record)
    require(isinstance(entries, list), f"schema missing record {record}")
    return [entry.split(":", 1)[0] for entry in entries]


def parse_ttrace(output: str) -> list[dict]:
    records: list[dict] = []
    for line in output.splitlines():
        if not line.startswith("TTRACE "):
            continue
        payload = line.removeprefix("TTRACE ")
        try:
            record = json.loads(payload)
        except json.JSONDecodeError as exc:
            fail(f"invalid TTRACE JSON: {exc}: {payload}")
        require(isinstance(record, dict), "TTRACE payload must be a JSON object")
        records.append(record)
    require(records, "S0 output did not emit TTRACE records")
    return records


def main() -> None:
    schema = read_json(SCHEMA)
    typed_contract = schema.get("typed_trace_contract", {})
    require(
        typed_contract.get("stage") == "s0_runtime_typed_trace_scaffold",
        "typed_trace_contract must honestly identify the S0 runtime scaffold stage",
    )
    required_records = typed_contract.get("required_records")
    require(required_records == REQUIRED_RECORDS, "typed trace required record list drifted")

    output = run_s0_if_needed()
    records = parse_ttrace(output)
    by_name: dict[str, dict] = {}
    for record in records:
        name = record.get("record")
        require(isinstance(name, str) and name, "TTRACE record missing record name")
        require(name not in by_name, f"duplicate TTRACE record {name}")
        by_name[name] = record

    missing = sorted(set(REQUIRED_RECORDS) - set(by_name))
    require(not missing, f"missing TTRACE records: {', '.join(missing)}")

    for name in REQUIRED_RECORDS:
        record = by_name[name]
        expected_fields = schema_fields(schema, name)
        actual_fields = sorted(key for key in record if key != "record")
        require(
            actual_fields == sorted(expected_fields),
            f"{name} fields drifted: actual={actual_fields} expected={sorted(expected_fields)}",
        )
        for field in expected_fields:
            require(isinstance(record[field], int), f"{name}.{field} must be an integer")
            require(record[field] >= 0, f"{name}.{field} must be non-negative")

    require(by_name["lnp64_retire_submit_t"]["tile_id"] == 0, "retire trace must identify tile 0")
    require(by_name["lnp64_thread_sched_t"]["tile_id"] == 0, "scheduler trace must identify tile 0")
    require(by_name["lnp64_event_t"]["tile_id"] == 0, "event trace must identify target tile 0")
    require(by_name["lnp64_fault_t"]["tile_id"] == 1, "fault trace must identify tile-local fault on tile 1")
    require(by_name["lnp64_tlb_cache_invalidate_t"]["tile_id"] == 0, "TLB/cache trace must identify tile 0")
    require(by_name["lnp64_coherence_txn_t"]["tile_id"] == 0, "coherence trace must identify tile 0")
    require(by_name["lnp64_trace_t"]["tile_id"] == 0, "watchdog/telemetry trace must identify tile 0")

    print("rtl typed trace contract ok")


if __name__ == "__main__":
    main()

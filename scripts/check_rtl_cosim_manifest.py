#!/usr/bin/env python3
"""Validate RTL/model co-simulation coverage metadata."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path(os.environ.get("LNP64_COSIM_MANIFEST", str(ROOT / "tests/traces/rtl_cosim_manifest.json")))
RANDOM_COSIM = Path(os.environ.get("LNP64_RANDOM_COSIM", str(ROOT / "scripts/run_rtl_random_cosim.sh")))
VERILATOR_COMMON = ROOT / "scripts/rtl_verilator_common.sh"
EXPECTED_SLICES = list(range(1, 16))
REQUIRED_TRACE_CATEGORIES = [
    "architectural_state",
    "result_codes",
    "event_records",
    "authority_generation_metadata",
]
REQUIRED_RANDOM_VARIES = {
    "m7_futex_atomic_seeded": ["timer_deadline"],
}


def text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def filelist_entries(path: Path) -> list[str]:
    entries: list[str] = []
    for raw in text(path).splitlines():
        line = raw.strip()
        if line and not line.startswith("#"):
            entries.append(line)
    return entries


def trace_lines(source: str) -> str:
    return "\n".join(line for line in source.splitlines() if "TRACE " in line)


def require_trace_category(source: str, category: str, tokens: list[str], context: str) -> None:
    require(trace_lines(source), f"{context} has no TRACE vocabulary")
    trace_text = source.lower()
    require(
        any(token.lower() in trace_text for token in tokens),
        f"{context} TRACE vocabulary does not cover {category}",
    )


def require_trace_markers(source: str, markers: list[str], context: str) -> None:
    for marker in markers:
        require(marker in source, f"{context} missing required trace marker {marker}")


def slice_id_from_top(top: str) -> int:
    match = re.fullmatch(r"lnp64_m([0-9]+)_tb", top)
    require(match is not None, f"unexpected RTL top name {top}")
    return int(match.group(1))


def main() -> None:
    require(RANDOM_COSIM.exists(), f"missing random co-sim driver {RANDOM_COSIM}")
    random_driver_text = text(RANDOM_COSIM)
    require(VERILATOR_COMMON.exists(), f"missing Verilator helper {VERILATOR_COMMON}")
    verilator_common_text = text(VERILATOR_COMMON)
    require(
        "scripts/check_rtl_cosim_manifest.py" in random_driver_text,
        "random co-sim driver must validate the manifest before running traces",
    )

    manifest = json.loads(text(MANIFEST))
    contract = manifest.get("trace_comparison_contract")
    require(isinstance(contract, dict), "manifest must define trace_comparison_contract")
    require(
        list(contract) == REQUIRED_TRACE_CATEGORIES,
        f"trace_comparison_contract must list categories {REQUIRED_TRACE_CATEGORIES} in order",
    )
    for category, tokens in contract.items():
        require(isinstance(tokens, list) and tokens, f"{category}: no trace tokens")
        for token in tokens:
            require(isinstance(token, str) and token, f"{category}: empty trace token")

    fixed = manifest.get("fixed_trace_gates", [])
    fixed_ids = [slice_id_from_top(gate.get("rtl_top", "")) for gate in fixed]
    require(fixed_ids == EXPECTED_SLICES, "expected fixed trace gates for M1 through M15 in order")
    for gate in fixed:
        slice_id = slice_id_from_top(gate["rtl_top"])
        script = ROOT / gate["script"]
        model = ROOT / gate["model"]
        filelist = ROOT / gate["filelist"]
        require(script.exists(), f"missing co-sim script {script}")
        require(model.exists(), f"missing executable model {model}")
        require(filelist.exists(), f"missing RTL filelist {filelist}")
        script_text = text(script)
        entries = filelist_entries(filelist)
        model_text = text(model)
        testbench_entry = f"rtl/sim/{gate['rtl_top']}.sv"
        testbench = ROOT / testbench_entry
        require(testbench.exists(), f"missing RTL testbench {testbench}")
        testbench_text = text(testbench)
        require(gate["model"] in script_text, f"{script} does not run {gate['model']}")
        require(
            f"--top-module {gate['rtl_top']}" in script_text,
            f"{script} does not build top {gate['rtl_top']}",
        )
        trace_driver_text = script_text
        if "rtl_run_seeded_trace_cosim" in script_text:
            require(
                "source scripts/rtl_verilator_common.sh" in script_text,
                f"{script} uses shared trace helper without sourcing it",
            )
            trace_driver_text += "\n" + verilator_common_text
        require("grep '^TRACE '" in trace_driver_text, f"{script} does not extract normalized TRACE lines")
        require("diff -u" in trace_driver_text, f"{script} does not diff model and RTL traces")
        require(
            f"LNP64-RTL-M{slice_id} PASS" in script_text,
            f"{script} does not require the M{slice_id} RTL PASS marker",
        )
        required_trace_markers = gate.get("required_trace_markers", [])
        require(
            isinstance(required_trace_markers, list),
            f"{gate['name']} required_trace_markers must be a list",
        )
        for marker in required_trace_markers:
            require(isinstance(marker, str) and marker, f"{gate['name']} has an empty trace marker")
        require_trace_markers(model_text, required_trace_markers, gate["model"])
        require_trace_markers(testbench_text, required_trace_markers, testbench_entry)
        require(
            testbench_entry in entries,
            f"{filelist} does not include RTL testbench {gate['rtl_top']}",
        )
        require(
            f"formal/rtl_assertions/lnp64_m{slice_id}_assertions.sv" in entries,
            f"{filelist} does not include M{slice_id} RTL assertions",
        )
        require(
            any(entry.startswith(f"rtl/engines/lnp64_m{slice_id}_") for entry in entries),
            f"{filelist} does not include an M{slice_id} RTL engine",
        )
        for category, tokens in contract.items():
            require_trace_category(model_text, category, tokens, f"{gate['model']}")
            require_trace_category(testbench_text, category, tokens, testbench_entry)

    random_gates = manifest.get("bounded_random_gates", [])
    require(len(random_gates) == len(EXPECTED_SLICES), "expected bounded random co-sim gates for M1 through M15")
    fixed_scripts = [gate["script"] for gate in fixed]
    default_seed_sets: set[tuple[int, ...]] = set()
    for gate in random_gates:
        script = ROOT / gate["script"]
        require(script.exists(), f"missing random co-sim script {script}")
        require(gate["script"] in fixed_scripts, f"{gate['name']} script is not a fixed trace gate")
        require(gate["script"] in random_driver_text, f"random co-sim driver does not invoke {gate['script']}")
        script_text = text(script)
        require(gate["seed_env"] in script_text, f"{script} does not consume {gate['seed_env']}")
        default_seeds = gate.get("default_seeds", [])
        require(len(default_seeds) >= 4, f"{gate['name']} needs multiple default seeds")
        require(all(isinstance(seed, int) and seed >= 0 for seed in default_seeds), f"{gate['name']} has invalid seeds")
        default_seed_sets.add(tuple(default_seeds))
        for varied in gate.get("varies", []):
            require(isinstance(varied, str) and varied, f"{gate['name']} has an empty varied field")
        for varied in REQUIRED_RANDOM_VARIES.get(gate["name"], []):
            require(varied in gate.get("varies", []), f"{gate['name']} must vary {varied}")
    require(len(default_seed_sets) == 1, "bounded random gates must share one default seed set")
    seed_string = " ".join(str(seed) for seed in next(iter(default_seed_sets)))
    require(seed_string in random_driver_text, "random co-sim driver default seeds do not match manifest")

    print("rtl cosim manifest ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Check the feature-gated top-level RTL program-test manifest."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(os.environ.get("LNP64_TOP_PROGRAM_ROOT", str(Path(__file__).resolve().parents[1])))
MANIFEST = Path(
    os.environ.get(
        "LNP64_TOP_PROGRAM_MANIFEST",
        str(ROOT / "tests/rtl/top_level_program_manifest.json"),
    )
)
RUN_DEMOS = ROOT / "scripts/run_demos.sh"


def fail(message: str) -> None:
    raise SystemExit(f"rtl top-level program manifest check failed: {message}")


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def text(path: Path) -> str:
    require(path.exists(), f"missing file {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def demo_c_sources_from_script(script_text: str) -> set[str]:
    block_match = re.search(r"non_network=\(\n(?P<body>.*?)\n\)", script_text, re.S)
    require(block_match is not None, "scripts/run_demos.sh must keep a non_network source list")
    sources = set(re.findall(r"\s+(demos/[A-Za-z0-9_./-]+\.c)", block_match.group("body")))
    sources.update(re.findall(r"cc\s+--toy-bootstrap\s+(demos/[A-Za-z0-9_./-]+\.c)\s+-o", script_text))
    return sources


def require_entry(entry: dict[str, object], source_kind: str) -> None:
    source = entry.get("source")
    require(isinstance(source, str) and source, f"{source_kind} entry has no source")
    require((ROOT / source).exists(), f"{source_kind} source is missing: {source}")
    features = entry.get("required_features")
    require(isinstance(features, list) and features, f"{source} must list required_features")
    require(all(isinstance(feature, str) and feature for feature in features), f"{source} has invalid required_features")
    status = entry.get("status")
    require(status in {"blocked_by_features", "active"}, f"{source} has invalid status {status}")
    if status == "blocked_by_features":
        require(features, f"{source} is blocked but has no required_features")
        require("rtl_gate" not in entry, f"{source} should not name an RTL gate until it is active")
    else:
        rtl_gate = entry.get("rtl_gate")
        require(isinstance(rtl_gate, str) and rtl_gate, f"{source} active entry must name rtl_gate")
        require((ROOT / rtl_gate).exists(), f"{source} rtl_gate is missing: {rtl_gate}")
        comparison = entry.get("comparison")
        require(
            comparison == "retire_trace_and_final_state",
            f"{source} active entry must compare retire_trace_and_final_state",
        )


def main() -> None:
    manifest = json.loads(text(MANIFEST))
    require(manifest.get("schema") == "lnp64_top_level_program_tests_v1", "unexpected manifest schema")
    require(manifest.get("stage") == "feature_gated_plan", "manifest must be a feature-gated plan")
    require(manifest.get("top") == "rtl/top/lnp64_top.sv", "manifest must target lnp64_top")
    require((ROOT / "rtl/top/lnp64_top.sv").exists(), "lnp64_top is missing")

    flat_hex_entries = manifest.get("flat_hex_programs")
    compiler_flat_entries = manifest.get("compiler_flat_programs")
    assembly_entries = manifest.get("assembly_programs")
    compiler_entries = manifest.get("compiler_generated_programs")
    require(isinstance(flat_hex_entries, list) and flat_hex_entries, "missing flat_hex_programs")
    require(isinstance(compiler_flat_entries, list) and compiler_flat_entries, "missing compiler_flat_programs")
    require(isinstance(assembly_entries, list) and assembly_entries, "missing assembly_programs")
    require(isinstance(compiler_entries, list) and compiler_entries, "missing compiler_generated_programs")
    active_flat_hex = 0
    for entry in flat_hex_entries:
        require(isinstance(entry, dict), "flat hex entry must be an object")
        require_entry(entry, "flat hex")
        if entry.get("status") == "active":
            active_flat_hex += 1
            gate_text = text(ROOT / str(entry["rtl_gate"]))
            require("program_input=" in gate_text, f"{entry['source']} active gate must accept a program input")
            generated_flat_hex = entry.get("generated_flat_hex")
            require(
                isinstance(generated_flat_hex, str) and generated_flat_hex.endswith(".hex"),
                f"{entry['source']} active entry must name generated_flat_hex",
            )
            require((ROOT / generated_flat_hex).exists(), f"{entry['source']} generated_flat_hex is missing")
            require("asm-flat-exec" in gate_text, f"{entry['source']} active gate must assemble source to flat hex")
            require("RTL_RETIRE" in gate_text and "EMULATOR_RETIRE" in gate_text, f"{entry['source']} gate must compare retire traces")
            require("RTL_FINAL" in gate_text and "EMULATOR_FINAL" in gate_text, f"{entry['source']} gate must compare final state")
    require(active_flat_hex >= 1, "manifest must keep at least one active top-level flat hex program")
    active_compiler_flat = 0
    for entry in compiler_flat_entries:
        require(isinstance(entry, dict), "compiler flat entry must be an object")
        require_entry(entry, "compiler flat")
        if entry.get("status") == "active":
            active_compiler_flat += 1
            gate_text = text(ROOT / str(entry["rtl_gate"]))
            require("*.c" in gate_text, f"{entry['source']} active gate must accept compiler input")
            require(
                (" cc " in gate_text or " cc --toy-bootstrap " in gate_text)
                and "asm-flat-exec" in gate_text,
                f"{entry['source']} active gate must compile C to flat hex",
            )
            require("RTL_RETIRE" in gate_text and "EMULATOR_RETIRE" in gate_text, f"{entry['source']} gate must compare retire traces")
            require("RTL_FINAL" in gate_text and "EMULATOR_FINAL" in gate_text, f"{entry['source']} gate must compare final state")
    require(active_compiler_flat >= 1, "manifest must keep at least one active compiler-generated top-level program")
    for entry in assembly_entries:
        require(isinstance(entry, dict), "assembly entry must be an object")
        require_entry(entry, "assembly")
    for entry in compiler_entries:
        require(isinstance(entry, dict), "compiler entry must be an object")
        require_entry(entry, "compiler")
        generated = entry.get("generated_assembly")
        require(isinstance(generated, str) and generated.endswith(".s"), f"{entry['source']} must name generated assembly")

    manifest_asm = {entry["source"] for entry in assembly_entries}
    actual_asm = {str(path.relative_to(ROOT)) for path in sorted((ROOT / "demos").glob("*.s"))}
    require(manifest_asm == actual_asm, f"demos/*.s coverage drifted: actual={sorted(actual_asm)} manifest={sorted(manifest_asm)}")

    manifest_c = {entry["source"] for entry in compiler_entries}
    script_c = demo_c_sources_from_script(text(RUN_DEMOS))
    require(manifest_c == script_c, f"compiler demo coverage drifted: script={sorted(script_c)} manifest={sorted(manifest_c)}")

    requirements = manifest.get("recurring_gate_requirements", [])
    require(isinstance(requirements, list) and len(requirements) >= 3, "missing recurring gate requirements")
    print("rtl top-level program manifest ok")


if __name__ == "__main__":
    main()

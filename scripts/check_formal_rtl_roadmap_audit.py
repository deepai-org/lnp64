#!/usr/bin/env python3
"""Check roadmap audit coverage and live-board completion evidence."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
AUDIT = ROOT / "formal_rtl_roadmap_audit.json"
CHECKLIST = Path(
    os.environ.get("LNP64_COMPLETION_CHECKLIST", str(ROOT / "formal_rtl_roadmap_completion_checklist.md"))
)
PROOF_MANIFEST = ROOT / "formal/proof_obligations_manifest.json"
TRACK_B_MANIFEST = ROOT / "rtl/track_b_blocks_manifest.json"
TRACK_C_MANIFEST = ROOT / "tests/traces/rtl_cosim_manifest.json"
TRACK_D_MANIFEST = ROOT / "fpga/bringup/lnp64_track_d_bringup.json"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--board-evidence",
        default=None,
        help="live board evidence JSON to validate in strict board-evidence mode",
    )
    parser.add_argument(
        "--require-board-evidence",
        action="store_true",
        help="fail unless a live board evidence JSON validates",
    )
    return parser.parse_args()


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def root_path(relative: str) -> Path:
    return ROOT / relative


def load_json(path: Path) -> dict:
    return json.loads(read_text(path))


def validate_references(audit: dict) -> dict:
    roadmap = root_path(audit["roadmap"])
    require(roadmap.exists(), f"missing roadmap {audit['roadmap']}")

    criteria = audit.get("criteria")
    require(isinstance(criteria, list) and criteria, "audit criteria must be a nonempty list")

    by_id: dict[str, dict] = {}
    for criterion in criteria:
        criterion_id = criterion.get("id")
        require(isinstance(criterion_id, str) and criterion_id, "criterion missing id")
        require(criterion_id not in by_id, f"duplicate audit criterion {criterion_id}")
        by_id[criterion_id] = criterion

        require(isinstance(criterion.get("title"), str) and criterion["title"], f"{criterion_id}: missing title")
        require(criterion.get("status") in {"implemented", "pending_hardware"}, f"{criterion_id}: invalid status")
        required_for_completion = criterion.get("required_for_completion")
        if criterion_id == "live_board_evidence":
            require(
                required_for_completion is False,
                f"{criterion_id}: live hardware evidence must be hardware-only in no-FPGA environments",
            )
        else:
            require(required_for_completion is True, f"{criterion_id}: must state required_for_completion")

        evidence_files = criterion.get("evidence_files")
        require(isinstance(evidence_files, list) and evidence_files, f"{criterion_id}: missing evidence files")
        for name in evidence_files:
            path = root_path(name)
            require(path.exists(), f"{criterion_id}: missing evidence file {name}")
            require(path.stat().st_size > 0, f"{criterion_id}: empty evidence file {name}")

        for checker_spec in checker_specs(criterion):
            checker = checker_spec["checker"]
            checker_path = root_path(checker)
            require(checker_path.exists(), f"{criterion_id}: missing checker {checker}")
            require(checker_path.stat().st_size > 0, f"{criterion_id}: empty checker {checker}")

    expected = {
        "s0_contract",
        "track_a_formal",
        "theorem_rtl_coupling",
        "formal_proof_manifest_checker_selftest",
        "track_b_rtl",
        "rtl_track_b_manifest_checker_selftest",
        "track_c_cosim",
        "rtl_cosim_manifest_checker_selftest",
        "track_d_fpga_smoke",
        "fpga_bringup_manifest_checker_selftest",
        "rtl_synth_constraints_checker_selftest",
        "fpga_report_checkers_selftest",
        "dockerized_gates",
        "rtl_dockerfiles_checker_selftest",
        "rtl_s0_contract_checker_selftest",
        "completion_checklist",
        "board_evidence_validator_selftest",
        "uart_byte_checker_selftest",
        "strict_roadmap_audit_selftest",
        "board_no_hardware_selftest",
        "first_milestone_s0",
        "second_milestone_m1_pingpong",
        "live_board_evidence",
    }
    missing = sorted(expected - set(by_id))
    require(not missing, f"audit omits roadmap criteria: {', '.join(missing)}")

    unexpected = sorted(set(by_id) - expected)
    require(not unexpected, f"audit has unknown roadmap criteria: {', '.join(unexpected)}")

    for criterion_id, criterion in by_id.items():
        status = criterion.get("status")
        if criterion_id == "live_board_evidence":
            require(
                status in {"pending_hardware", "implemented"},
                f"{criterion_id}: expected pending_hardware or implemented, got {status}",
            )
        else:
            require(status == "implemented", f"{criterion_id}: expected status implemented, got {status}")

    return by_id


def checker_specs(criterion: dict) -> list[dict[str, str]]:
    specs: list[dict[str, str]] = []
    checker = criterion.get("checker")
    if checker is not None:
        require(isinstance(checker, str) and checker, f"{criterion.get('id')}: invalid checker")
        spec = {"checker": checker}
        success_line = criterion.get("success_line")
        if success_line is not None:
            require(
                isinstance(success_line, str) and success_line,
                f"{criterion.get('id')}: invalid success_line",
            )
            spec["success_line"] = success_line
        specs.append(spec)

    additional = criterion.get("additional_checkers", [])
    require(isinstance(additional, list), f"{criterion.get('id')}: additional_checkers must be a list")
    for spec in additional:
        require(isinstance(spec, dict), f"{criterion.get('id')}: additional checker must be an object")
        checker = spec.get("checker")
        success_line = spec.get("success_line")
        require(isinstance(checker, str) and checker, f"{criterion.get('id')}: invalid additional checker")
        require(
            isinstance(success_line, str) and success_line,
            f"{criterion.get('id')}: invalid additional checker success_line",
        )
        specs.append({"checker": checker, "success_line": success_line})
    return specs


def run_checker(checker: str) -> subprocess.CompletedProcess[str]:
    checker_path = root_path(checker)
    command = [str(checker_path)]
    if checker_path.suffix == ".py":
        command = [sys.executable, str(checker_path)]
    result = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        text=True,
        capture_output=True,
    )
    result.stdout = (result.stdout or "") + (result.stderr or "")
    return result


def validate_implemented_checkers(by_id: dict[str, dict]) -> None:
    for criterion_id, criterion in by_id.items():
        if criterion_id == "live_board_evidence":
            continue
        if criterion.get("status") != "implemented":
            continue
        if (
            criterion_id == "strict_roadmap_audit_selftest"
            and os.environ.get("LNP64_SKIP_STRICT_ROADMAP_AUDIT_SELFTEST") == "1"
        ):
            continue
        for spec in checker_specs(criterion):
            result = run_checker(spec["checker"])
            if result.returncode != 0:
                if result.stdout:
                    print(result.stdout, end="", file=sys.stderr)
                raise SystemExit(
                    f"{criterion_id}: checker {spec['checker']} failed with exit code {result.returncode}"
                )
            success_line = spec.get("success_line")
            if success_line is not None:
                require(
                    success_line in result.stdout,
                    f"{criterion_id}: checker {spec['checker']} did not print {success_line}",
                )


def validate_required_strings(audit: dict) -> None:
    required_strings = audit.get("required_strings_by_file", {})
    require(isinstance(required_strings, dict) and required_strings, "audit must include required_strings_by_file")
    for file_name, strings in required_strings.items():
        path = root_path(file_name)
        require(path.exists(), f"missing required-string file {file_name}")
        text = read_text(path)
        require(isinstance(strings, list) and strings, f"{file_name}: no required strings")
        for needle in strings:
            require(needle in text, f"{file_name}: missing required wiring string {needle}")


def normalized_markdown(path: Path) -> str:
    return read_text(path).replace("`", "").lower()


def validate_completion_checklist() -> None:
    require(CHECKLIST.exists(), f"missing completion checklist {CHECKLIST}")
    checklist = normalized_markdown(CHECKLIST)

    proof_manifest = load_json(PROOF_MANIFEST)
    obligations = proof_manifest.get("obligations")
    require(isinstance(obligations, list) and obligations, "proof manifest has no obligations")
    obligation_ids = [entry.get("id") for entry in obligations]
    require(
        [f"A{i}" for i in range(1, 11)] == [item for item in obligation_ids if str(item).startswith("A")],
        "proof manifest must enumerate A1-A10 in order",
    )
    for obligation_id in (f"A{i}" for i in range(1, 11)):
        require(f"| {obligation_id.lower()} " in checklist, f"completion checklist omits {obligation_id}")

    track_b = load_json(TRACK_B_MANIFEST)
    blocks = track_b.get("blocks")
    require(isinstance(blocks, list) and blocks, "Track B manifest has no blocks")
    block_ids = [block.get("id") for block in blocks]
    require(block_ids == ["B0"] + [f"B{i}" for i in range(1, 15)], "Track B manifest must enumerate B0-B14")
    for block in blocks:
        label = f"{block['id']} {block['title']}".replace("`", "").lower()
        require(label in checklist, f"completion checklist omits Track B row {label}")

    track_c = load_json(TRACK_C_MANIFEST)
    contract = track_c.get("trace_comparison_contract")
    require(isinstance(contract, dict) and contract, "Track C manifest has no trace comparison contract")
    for row in (
        "same input vector in emulator/model and rtl simulation",
        "compare architectural state",
        "compare result codes",
        "compare event records",
        "compare fdr/generation/authority metadata",
        "random but bounded traces from models",
        "verilator for fast ci",
        "fpga simulation and synthesis checks",
    ):
        require(row in checklist, f"completion checklist omits Track C row {row}")
    for category in contract:
        require(category.lower() in checklist, f"completion checklist omits Track C category {category}")

    track_d = load_json(TRACK_D_MANIFEST)
    steps = track_d.get("steps")
    require(isinstance(steps, list) and steps, "Track D manifest has no steps")
    require([step.get("id") for step in steps] == list(range(1, 18)), "Track D manifest must enumerate steps 1-17")
    for step in steps:
        label = f"{step['id']}. {step['title']}".replace("`", "").lower()
        require(label in checklist, f"completion checklist omits Track D row {label}")

    for row in (
        "first milestone required slice",
        "first milestone proof targets",
        "first milestone expected demo",
        "second milestone required slice",
        "second milestone proof targets",
        "second milestone expected demo",
    ):
        require(row in checklist, f"completion checklist omits milestone row {row}")


def validate_board_evidence(criterion: dict, evidence_arg: str | None, strict: bool) -> bool:
    evidence_name = evidence_arg or os.environ.get("LNP64_BOARD_EVIDENCE") or criterion["default_evidence"]
    evidence = root_path(evidence_name) if not Path(evidence_name).is_absolute() else Path(evidence_name)
    if not strict:
        return evidence.exists()

    checker = root_path(criterion["checker"])
    require(evidence.exists(), f"missing required live board evidence {evidence}")
    subprocess.run([str(checker), str(evidence)], cwd=ROOT, check=True)
    return True


def main() -> None:
    args = parse_args()
    audit = json.loads(read_text(AUDIT))
    require(audit.get("name") == "lnp64_formal_rtl_roadmap_audit", "unexpected audit name")

    by_id = validate_references(audit)
    validate_required_strings(audit)
    validate_completion_checklist()
    validate_implemented_checkers(by_id)

    strict = args.require_board_evidence or os.environ.get("LNP64_REQUIRE_BOARD_EVIDENCE") == "1"
    board = by_id["live_board_evidence"]
    if board.get("status") == "implemented":
        strict = True
    board_ok = validate_board_evidence(board, args.board_evidence, strict)

    if board.get("status") == "implemented":
        require(board_ok, "live board evidence is implemented but evidence did not validate")

    if strict:
        print("formal RTL roadmap audit ok")
    elif board_ok:
        print("formal RTL roadmap audit ok (board evidence present but not required)")
    else:
        print("formal RTL roadmap audit ok (live board evidence hardware-only)")


if __name__ == "__main__":
    main()

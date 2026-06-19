#!/usr/bin/env python3
"""Check Dockerfile-backed RTL/proof/synth/board command paths."""

from __future__ import annotations

import os
import re
from pathlib import Path


ROOT = Path(os.environ.get("LNP64_DOCKERFILES_ROOT", str(Path(__file__).resolve().parents[1])))


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def require_all(text: str, needles: list[str], label: str) -> None:
    for needle in needles:
        require(needle in text, f"{label}: missing {needle}")


def require_package(dockerfile: str, package: str, label: str) -> None:
    require(re.search(rf"(?m)^\s*{re.escape(package)}\s*\\?$", dockerfile), f"{label}: missing package {package}")


def check_proof() -> None:
    dockerfile = read_text("Dockerfile.rtl-proof")
    wrapper = read_text("scripts/run_rtl_proof_docker.sh")
    gate = read_text("scripts/run_rtl_proof_gates.sh")
    m1_wrapper = read_text("scripts/run_rtl_m1_refinement_docker.sh")
    m1_gate = read_text("scripts/run_rtl_m1_refinement_gate.sh")

    require_all(
        dockerfile,
        [
            "FROM debian:bookworm",
            "ARG LEAN_TOOLCHAIN",
            "ENV LNP64_REQUIRE_LEAN=1",
            "elan-init.sh",
            "lean --version",
            "bash scripts/run_rtl_proof_gates.sh",
            'CMD ["bash", "scripts/run_rtl_proof_gates.sh"]',
        ],
        "Dockerfile.rtl-proof",
    )
    for package in ("bash", "build-essential", "curl", "git", "python3", "verilator"):
        require_package(dockerfile, package, "Dockerfile.rtl-proof")

    require_all(
        wrapper,
        [
            "docker build",
            "-f Dockerfile.rtl-proof",
            "LEAN_TOOLCHAIN",
            "RUN_RTL_PROOF_GATES",
            "LNP64_RTL_PROOF_BUILD_GATES",
            "LNP64_RTL_PROOF_SKIP_BUILD",
            "using existing RTL/proof Docker image",
            "docker run --rm",
            "-e LNP64_REQUIRE_LEAN=1",
            "LNP64_RTL_PROOF_RANDOM_COSIM",
            "LNP64_RTL_RANDOM_COSIM_JOBS",
            "-v \"$root:/work\"",
            "-w /work",
            "bash scripts/run_rtl_proof_gates.sh",
        ],
        "scripts/run_rtl_proof_docker.sh",
    )
    require_all(
        gate,
        [
            "LNP64_RTL_PROOF_RANDOM_COSIM",
            "scripts/check_rtl_cosim_manifest.py",
            "bash scripts/run_rtl_random_cosim.sh",
            "LNP64_RTL_RANDOM_COSIM_JOBS",
            "rtl random cosim skipped",
        ],
        "scripts/run_rtl_proof_gates.sh",
    )
    require_all(
        m1_wrapper,
        [
            "docker build",
            "-f Dockerfile.rtl-proof",
            "LEAN_TOOLCHAIN",
            "RUN_RTL_PROOF_GATES",
            "LNP64_RTL_PROOF_SKIP_BUILD",
            "docker run --rm",
            "-e LNP64_REQUIRE_LEAN=1",
            "LNP64_M1_TYPED_COMMIT_SEEDS",
            "-v \"$root:/work\"",
            "-w /work",
            "bash scripts/run_rtl_m1_refinement_gate.sh",
        ],
        "scripts/run_rtl_m1_refinement_docker.sh",
    )
    require_all(
        m1_gate,
        [
            "formal/M1TransitionInvariantModel.lean",
            "lean \"$lean_file\"",
            "scripts/check_rtl_shared_schema.py",
            "scripts/check_theorem_rtl_coupling.py",
            "formal/m1_model.py",
            "bash scripts/run_rtl_m1.sh",
            "LNP64_M1_TYPED_COMMIT_USE_EXISTING=1",
            "LNP64_M1_TYPED_COMMIT_LOG=\"$m1_log\"",
            "scripts/check_rtl_m1_typed_commit_trace.py",
            "scripts/test_rtl_m1_typed_commit_checker.py",
            "scripts/test_rtl_m1_schema_checker.py",
            "rtl m1 refinement gate ok",
        ],
        "scripts/run_rtl_m1_refinement_gate.sh",
    )


def check_synth() -> None:
    dockerfile = read_text("Dockerfile.rtl-synth")
    wrapper = read_text("scripts/run_rtl_synth_docker.sh")

    require_all(
        dockerfile,
        [
            "FROM debian:bookworm",
            "ARG RUN_RTL_SYNTH_SMOKE",
            "bash scripts/run_rtl_synth_gates.sh",
            'CMD ["bash", "scripts/run_rtl_synth_gates.sh"]',
        ],
        "Dockerfile.rtl-synth",
    )
    for package in (
        "bash",
        "fpga-icestorm",
        "fpga-icestorm-chipdb",
        "g++",
        "make",
        "nextpnr-ice40",
        "python3",
        "verilator",
        "yosys",
    ):
        require_package(dockerfile, package, "Dockerfile.rtl-synth")

    require_all(
        wrapper,
        [
            "docker build",
            "-f Dockerfile.rtl-synth",
            "docker run --rm",
            "-v \"$root:/work\"",
            "-w /work",
            "bash scripts/run_rtl_synth_gates.sh",
        ],
        "scripts/run_rtl_synth_docker.sh",
    )


def check_board() -> None:
    dockerfile = read_text("Dockerfile.rtl-board")
    wrapper = read_text("scripts/run_rtl_board_docker.sh")
    gate = read_text("scripts/run_rtl_board_ice40_s0.sh")

    require_all(
        dockerfile,
        [
            "FROM debian:bookworm",
            'CMD ["bash", "scripts/run_rtl_board_ice40_s0.sh"]',
        ],
        "Dockerfile.rtl-board",
    )
    for package in (
        "bash",
        "fpga-icestorm",
        "fpga-icestorm-chipdb",
        "nextpnr-ice40",
        "python3",
        "python3-serial",
        "usbutils",
        "verilator",
        "yosys",
    ):
        require_package(dockerfile, package, "Dockerfile.rtl-board")

    require_all(
        wrapper,
        [
            "docker build",
            "-f Dockerfile.rtl-board",
            "docker run --rm",
            "--privileged",
            "resolved_uart=\"$(readlink -f \"$uart_device\")\"",
            "Board UART device does not look like a serial TTY",
            "--device \"$resolved_uart:$resolved_uart\"",
            "-e LNP64_BOARD_UART_DEVICE=\"$resolved_uart\"",
            "-e LNP64_BOARD_EVIDENCE_OUT=",
            "-e LNP64_ICE40_BIN_OUT=",
            "-v \"$root:/work\"",
            "-w /work",
            "bash scripts/run_rtl_board_ice40_s0.sh",
        ],
        "scripts/run_rtl_board_docker.sh",
    )
    require_all(
        gate,
        [
            "run_rtl_fpga_ice40_s0.sh",
            "run_rtl_board_preflight.sh",
            "iceprog",
            "check_uart_byte.py",
            "check_board_evidence.py",
            "rtl board ice40 s0 live uart ok",
        ],
        "scripts/run_rtl_board_ice40_s0.sh",
    )


def check_aggregate_and_docs() -> None:
    all_gates = read_text("scripts/run_all_gates.sh")
    audit = read_text("scripts/run_formal_rtl_roadmap_audit.sh")
    readme = read_text("README.md")

    require_all(
        all_gates,
        [
            "scripts/run_rtl_proof_docker.sh",
            "scripts/run_rtl_synth_docker.sh",
            "scripts/run_formal_rtl_roadmap_audit.sh",
        ],
        "scripts/run_all_gates.sh",
    )
    require_all(
        audit,
        [
            "--docker-rerun",
            "--docker-build",
            "docker run --rm",
            "scripts/run_rtl_proof_docker.sh",
            "scripts/run_rtl_synth_docker.sh",
        ],
        "scripts/run_formal_rtl_roadmap_audit.sh",
    )
    require_all(
        readme,
        [
            "RTL And Proof Gates",
            "bash scripts/run_rtl_proof_docker.sh",
            "LNP64_RTL_PROOF_SKIP_BUILD=1 bash scripts/run_rtl_m1_refinement_docker.sh",
            "LNP64_RTL_PROOF_RANDOM_COSIM=0 bash scripts/run_rtl_proof_docker.sh",
            "LNP64_RTL_PROOF_SKIP_BUILD=1 LNP64_RTL_PROOF_RANDOM_COSIM=0 bash scripts/run_rtl_proof_docker.sh",
            "LNP64_RTL_PROOF_BUILD_GATES=1",
            "bash scripts/run_rtl_synth_docker.sh",
            "bash scripts/run_rtl_proof_gates.sh",
            "bash scripts/run_rtl_synth_gates.sh",
            "scripts/run_rtl_m1_refinement_gate.sh",
            "scripts/check_theorem_rtl_coupling.py",
            "FPGA Board Note",
            "Dockerized RTL/proof and synthesis/FPGA-smoke gates are the reproducible evidence path",
        ],
        "README.md",
    )


def main() -> None:
    check_proof()
    check_synth()
    check_board()
    check_aggregate_and_docs()
    print("rtl Dockerfile command paths ok")


if __name__ == "__main__":
    main()

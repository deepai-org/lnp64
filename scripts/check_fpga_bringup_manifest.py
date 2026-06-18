#!/usr/bin/env python3
"""Check the Track D FPGA bring-up manifest against repository artifacts."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path(os.environ.get("LNP64_FPGA_BRINGUP_MANIFEST", str(ROOT / "fpga/bringup/lnp64_track_d_bringup.json")))
ROADMAP = Path(os.environ.get("LNP64_ROADMAP", str(ROOT / "formal_rtl_codesign_roadmap.md")))
SYNTH_SMOKE = Path(os.environ.get("LNP64_SYNTH_SMOKE", str(ROOT / "scripts/run_rtl_synth_smoke.sh")))
ALLOWED_STATUS = {"implemented", "reserved_stub", "reserved_later"}
EXPECTED_IDS = list(range(1, 18))

REQUIRED_STEP_MARKERS = {
    1: [
        "LNP64-RTL-S0-FPGA PASS",
        "lnp64_s0_fpga_top",
        "lnp64_fail_closed_engine",
    ],
    2: [
        "LNP64_OP_ENV_GET",
        "LNP64_OP_UNSUPPORTED",
        "ENV_GET did not report expected S0 feature bits",
    ],
    3: [
        "LNP64_OP_LD",
        "LNP64_OP_ST",
        "SRAM LD/ST path did not roundtrip the ALU value",
    ],
    4: [
        "UART boot/status byte was not observed",
        "uart_byte",
    ],
    5: [
        "LNP64_OP_NOP",
        "LNP64_OP_LI32",
        "PID 1 retired too few S0 instructions",
    ],
    6: [
        "TRACE cap_dup",
        "TRACE stale_pull",
    ],
    7: [
        "TRACE await",
        "TRACE push",
        "TRACE pull",
    ],
    8: [
        "TRACE gate_call",
        "TRACE clone",
        "TRACE join",
    ],
    9: [
        "TRACE store_denied",
        "TRACE exec_fault",
        "TRACE tlb_invalidate",
    ],
    10: [
        "TRACE ddr_write",
        "TRACE ddr_read",
        "TRACE barrier",
    ],
    11: [
        "TRACE dma_copy",
        "TRACE dma_fill",
        "TRACE coherence_flush",
    ],
    12: [
        "TRACE envelope",
        "TRACE ns_dispatch",
        "TRACE cap_proposal",
    ],
    13: [
        "TRACE futex_wait",
        "TRACE futex_wake",
        "TRACE alloc",
        "TRACE free",
    ],
    14: [
        "TRACE boot_image",
        "TRACE block_write",
        "TRACE barrier",
    ],
    15: [
        "TRACE packet_steer",
        "TRACE ipc_steer",
        "TRACE verifier",
    ],
    16: [
        "TRACE ecc_corrected",
        "TRACE watchdog_timeout",
        "TRACE quote_stub",
    ],
    17: [
        "TRACE enumerate",
        "TRACE iommu_dma",
        "TRACE raw_pcie",
    ],
}


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def module_exists(module: str) -> bool:
    pattern = re.compile(rf"\bmodule\s+{re.escape(module)}\b")
    return any(
        pattern.search(read_text(path))
        for source_glob in ("rtl/**/*.sv", "fpga/rtl/**/*.sv")
        for path in ROOT.glob(source_glob)
    )


def filelist_paths(filelist: Path) -> list[Path]:
    paths: list[Path] = []
    for raw in read_text(filelist).splitlines():
        line = raw.strip()
        if line and not line.startswith("#"):
            paths.append(ROOT / line)
    return paths


def main() -> None:
    require(MANIFEST.exists(), f"missing FPGA bring-up manifest {MANIFEST}")
    require(ROADMAP.exists(), f"missing roadmap {ROADMAP}")
    require(SYNTH_SMOKE.exists(), f"missing synthesis smoke gate {SYNTH_SMOKE}")

    manifest = json.loads(read_text(MANIFEST))
    require(manifest.get("track") == "Track D: FPGA Bring-Up", "manifest must name Track D")
    synth_smoke_text = read_text(SYNTH_SMOKE)
    fpga_uart_gate_text = read_text(ROOT / "scripts/run_rtl_fpga_uart_s0.sh")
    fpga_gate_text = read_text(ROOT / "scripts/run_rtl_fpga_ice40_s0.sh")

    steps = manifest.get("steps")
    require(isinstance(steps, list), "manifest steps must be a list")
    ids = [step.get("id") for step in steps]
    require(ids == EXPECTED_IDS, f"Track D step ids must be {EXPECTED_IDS}")

    roadmap_text = read_text(ROADMAP).replace("`", "")
    for step in steps:
        title = step.get("title")
        status = step.get("status")
        scripts = step.get("gate_scripts")
        filelists = step.get("filelists")
        modules = step.get("modules")

        require(isinstance(title, str) and title, f"step {step.get('id')} missing title")
        require(title in roadmap_text, f"step title is not present in roadmap: {title}")
        require(status in ALLOWED_STATUS, f"step {step['id']} has invalid status {status}")
        require(isinstance(scripts, list) and scripts, f"step {step['id']} has no gate scripts")
        require(isinstance(filelists, list) and filelists, f"step {step['id']} has no filelists")
        require(isinstance(modules, list) and modules, f"step {step['id']} has no modules")

        for script in scripts:
            path = ROOT / script
            require(path.exists(), f"step {step['id']} missing gate script {script}")
            require(path.stat().st_size > 0, f"step {step['id']} empty gate script {script}")

        step_source_paths: list[Path] = []
        for filelist_name in filelists:
            filelist = ROOT / filelist_name
            require(filelist.exists(), f"step {step['id']} missing filelist {filelist_name}")
            sources = filelist_paths(filelist)
            require(sources, f"step {step['id']} empty filelist {filelist_name}")
            for source in sources:
                require(source.exists(), f"step {step['id']} missing filelist source {source}")
            step_source_paths.extend(sources)

        for module in modules:
            require(module_exists(module), f"step {step['id']} missing RTL module {module}")

        if status == "implemented":
            step_source_text = "\n".join(read_text(source) for source in sorted(set(step_source_paths)))
            for marker in REQUIRED_STEP_MARKERS[step["id"]]:
                require(marker in step_source_text, f"step {step['id']} missing smoke marker {marker}")

            require(
                any(script.startswith("scripts/run_rtl_m") or script == "scripts/run_rtl_s0.sh" for script in scripts),
                f"implemented step {step['id']} must name an RTL gate",
            )
            for filelist_name in filelists:
                require(
                    filelist_name in synth_smoke_text
                    or filelist_name in fpga_uart_gate_text
                    or filelist_name in fpga_gate_text,
                    f"implemented step {step['id']} filelist {filelist_name} is not covered by synthesis/FPGA gates",
                )
        else:
            require(
                "stub" in status or "later" in status,
                f"reserved step {step['id']} must make its non-implemented status explicit",
            )

    print("fpga bringup manifest ok")


if __name__ == "__main__":
    main()

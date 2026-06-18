#!/usr/bin/env python3
"""Check the RTL synthesis/FPGA smoke constraint manifest."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path(os.environ.get("LNP64_SYNTH_CONSTRAINTS_MANIFEST", str(ROOT / "fpga/constraints/lnp64_s0_smoke.json")))
EXPECTED_VERTICAL_SLICE_TOPS = [
    "lnp64_m1_pingpong",
    "lnp64_m2_gate",
    "lnp64_m3_process",
    "lnp64_m4_vma",
    "lnp64_m5_dma",
    "lnp64_m6_service",
    "lnp64_m7_futex_atomic",
    "lnp64_m8_heap",
    "lnp64_m9_classifier_servicelet",
    "lnp64_m10_ras",
    "lnp64_m11_ddr_metadata",
    "lnp64_m12_storage_barrier",
    "lnp64_m13_pcie_iommu",
    "lnp64_m14_resource_domain_policy",
    "lnp64_m15_object_profiles",
]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def filelist_paths(filelist: Path) -> list[Path]:
    paths: list[Path] = []
    for raw in read_text(filelist).splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        paths.append(ROOT / line)
    return paths


def module_body(source: str, module: str) -> str:
    pattern = re.compile(rf"\bmodule\s+{re.escape(module)}\b(?P<body>.*?);", re.S)
    match = pattern.search(source)
    require(match is not None, f"missing module declaration for {module}")
    return match.group("body")


def main() -> None:
    manifest = json.loads(read_text(MANIFEST))
    top = manifest["top"]
    filelist = ROOT / manifest["source_filelist"]
    xdc = ROOT / manifest["xdc"]

    require(filelist.exists(), f"missing source filelist {filelist}")
    require(xdc.exists(), f"missing XDC constraint file {xdc}")

    sources = filelist_paths(filelist)
    for source in sources:
        require(source.exists(), f"missing RTL source {source}")

    combined = "\n".join(read_text(source) for source in sources)
    top_ports = module_body(combined, top)
    for port in manifest["required_top_ports"]:
        require(re.search(rf"\b{re.escape(port)}\b", top_ports), f"top port not constrained in manifest: {port}")

    xdc_text = read_text(xdc)
    clock = manifest["clock"]
    reset = manifest["reset"]
    require("create_clock" in xdc_text, "XDC must define a create_clock")
    require("TBD" not in xdc_text, "XDC must not contain placeholder TBD constraints")
    require(clock["port"] in xdc_text, f"XDC does not mention clock port {clock['port']}")
    require(reset["port"] in xdc_text, f"XDC does not mention reset port {reset['port']}")
    require(clock["frequency_hz"] > 0, "clock frequency must be positive")
    require(reset["active"] in {"low", "high"}, "reset active level must be low or high")

    for step in manifest["bringup_smoke_steps"]:
        require(isinstance(step, str) and step, "empty FPGA smoke step")

    bitstream = manifest.get("fpga_bitstream_smoke")
    require(isinstance(bitstream, dict), "missing FPGA bitstream smoke metadata")
    bitstream_filelist = ROOT / bitstream["source_filelist"]
    bitstream_script = ROOT / bitstream["gate_script"]
    bitstream_wrapper = ROOT / bitstream["wrapper"]
    bitstream_pcf = ROOT / bitstream["pcf"]
    require(bitstream_filelist.exists(), f"missing FPGA bitstream filelist {bitstream_filelist}")
    require(bitstream_script.exists(), f"missing FPGA bitstream gate {bitstream_script}")
    require(bitstream_wrapper.exists(), f"missing FPGA bitstream wrapper {bitstream_wrapper}")
    require(bitstream_pcf.exists(), f"missing FPGA bitstream PCF {bitstream_pcf}")
    require(bitstream.get("target") == "ice40-hx8k-ct256", "unexpected FPGA bitstream target")
    require(bitstream.get("top") == "lnp64_s0_fpga_top", "unexpected FPGA bitstream top")
    require(bitstream.get("minimum_frequency_mhz") == 12, "unexpected FPGA bitstream timing target")
    bitstream_script_text = read_text(bitstream_script)
    for tool in ("synth_ice40", "nextpnr-ice40", "icepack", "icetime", "check_ice40_report.py", "check_icetime_report.py"):
        require(tool in bitstream_script_text, f"FPGA bitstream gate does not invoke {tool}")
    require("--pcf" in bitstream_script_text, "FPGA bitstream gate does not pass a PCF to nextpnr")
    pcf_text = read_text(bitstream_pcf)
    for port in ("clk", "reset_n", "uart_tx"):
        require(re.search(rf"(?m)^set_io\s+{re.escape(port)}\s+\S+", pcf_text), f"PCF missing {port}")
    for bit in range(6):
        require(
            re.search(rf"(?m)^set_io\s+status_led\[{bit}\]\s+\S+", pcf_text),
            f"PCF missing status_led[{bit}]",
        )
    bitstream_sources = filelist_paths(bitstream_filelist)
    require(bitstream_sources, f"empty FPGA bitstream filelist {bitstream_filelist}")
    for source in bitstream_sources:
        require(source.exists(), f"missing FPGA bitstream source {source}")
    bitstream_combined = "\n".join(read_text(source) for source in bitstream_sources)
    require(
        re.search(r"\bmodule\s+lnp64_s0_fpga_top\b", bitstream_combined),
        "missing FPGA wrapper top module",
    )
    require(
        re.search(r"\bmodule\s+lnp64_s0_uart_tx\b", bitstream_combined),
        "missing FPGA UART transmitter module",
    )

    uart_sim = manifest.get("fpga_uart_sim")
    require(isinstance(uart_sim, dict), "missing FPGA UART simulation metadata")
    uart_sim_filelist = ROOT / uart_sim["source_filelist"]
    uart_sim_script = ROOT / uart_sim["gate_script"]
    require(uart_sim.get("top") == "lnp64_s0_fpga_tb", "unexpected FPGA UART simulation top")
    require(uart_sim.get("expected_uart_hex") == "53", "unexpected FPGA UART simulation byte")
    require(uart_sim.get("expected_leds") == "111111", "unexpected FPGA UART simulation LED value")
    require(uart_sim.get("expected_success") == "rtl fpga uart s0 gate ok", "unexpected FPGA UART simulation success line")
    require(uart_sim_filelist.exists(), f"missing FPGA UART simulation filelist {uart_sim_filelist}")
    require(uart_sim_script.exists(), f"missing FPGA UART simulation gate {uart_sim_script}")
    uart_sim_sources = filelist_paths(uart_sim_filelist)
    require(uart_sim_sources, f"empty FPGA UART simulation filelist {uart_sim_filelist}")
    for source in uart_sim_sources:
        require(source.exists(), f"missing FPGA UART simulation source {source}")
    uart_sim_combined = "\n".join(read_text(source) for source in uart_sim_sources)
    require(re.search(r"\bmodule\s+lnp64_s0_fpga_tb\b", uart_sim_combined), "missing FPGA UART testbench")
    require("8'h53" in uart_sim_combined, "FPGA UART testbench must check boot byte 0x53")
    require("6'b111111" in uart_sim_combined, "FPGA UART testbench must check status LEDs")
    uart_sim_script_text = read_text(uart_sim_script)
    for required in ("verilator", "s0_fpga_uart_filelist.f", "LNP64-RTL-S0-FPGA PASS"):
        require(required in uart_sim_script_text, f"FPGA UART simulation gate does not mention {required}")
    synth_gate_text = read_text(ROOT / "scripts/run_rtl_synth_gates.sh")
    synth_smoke_text = read_text(ROOT / "scripts/run_rtl_synth_smoke.sh")
    vertical_yosys_text = read_text(ROOT / "scripts/run_rtl_yosys_vertical_slices.sh")
    require(uart_sim["gate_script"] in synth_gate_text, "synthesis gates do not run FPGA UART simulation")
    require("scripts/run_rtl_yosys_vertical_slices.sh" in synth_smoke_text, "synthesis smoke gate does not run vertical Yosys gate")

    board = manifest.get("board_live_validation")
    require(isinstance(board, dict), "missing board live validation metadata")
    board_dockerfile = ROOT / board["dockerfile"]
    board_docker_gate = ROOT / board["docker_gate_script"]
    board_preflight = ROOT / board["preflight_script"]
    board_gate = ROOT / board["gate_script"]
    uart_checker = ROOT / board["uart_checker"]
    evidence_checker = ROOT / board["evidence_checker"]
    require(board.get("target") == bitstream.get("target"), "board target must match bitstream target")
    require(board.get("programmer") == "iceprog", "board validation must name iceprog")
    require(board.get("evidence_schema") == "lnp64_board_ice40_s0_v1", "unexpected board evidence schema")
    require(
        board.get("direct_default_evidence") == "/tmp/lnp64-board-ice40-s0-evidence.json",
        "unexpected direct board evidence path",
    )
    require(
        board.get("docker_default_evidence") == "/work/build/lnp64-board-ice40-s0-evidence.json",
        "unexpected Docker board evidence path",
    )
    require(board.get("uart_baud") == 115200, "unexpected board UART baud")
    require(board.get("expected_uart_hex") == "53", "unexpected board UART byte")
    require(board.get("expected_preflight_success") == "rtl board ice40 s0 preflight ok", "unexpected board preflight success line")
    require(board.get("expected_success") == "rtl board ice40 s0 live uart ok", "unexpected board success line")
    require("LNP64_BOARD_UART_DEVICE" in board.get("required_env", []), "board validation must require UART device")
    for path in (board_dockerfile, board_docker_gate, board_preflight, board_gate, uart_checker, evidence_checker):
        require(path.exists(), f"missing board validation artifact {path.relative_to(ROOT)}")
        require(path.stat().st_size > 0, f"empty board validation artifact {path.relative_to(ROOT)}")
    dockerfile_text = read_text(board_dockerfile)
    require("fpga-icestorm" in dockerfile_text, "board Dockerfile must install IceStorm")
    require("python3-serial" in dockerfile_text, "board Dockerfile must install pyserial")
    board_gate_text = read_text(board_gate)
    for required in (
        "iceprog",
        "check_uart_byte.py",
        "check_board_evidence.py",
        "run_rtl_fpga_ice40_s0.sh",
        "run_rtl_board_preflight.sh",
        "readlink -f",
        "invalid LNP64_BOARD_UART_EXPECT_HEX",
        "LNP64_BOARD_UART_EXPECT_HEX is not a byte",
        "LNP64_BOARD_EVIDENCE_OUT",
        "/tmp/lnp64-board-ice40-s0-evidence.json",
        "LNP64_BOARD_PREFLIGHT_LOG",
        "preflight_log",
        "LNP64_BOARD_UART_DEVICE",
    ):
        require(required in board_gate_text, f"board gate does not mention {required}")
    board_preflight_text = read_text(board_preflight)
    for required in (
        "iceprog",
        "-t",
        "LNP64_BOARD_UART_DEVICE",
        "LNP64_ICEPROG_DEVICE",
        "LNP64_BOARD_SKIP_PROGRAMMER_PROBE",
        "rtl board ice40 s0 preflight ok",
    ):
        require(required in board_preflight_text, f"board preflight does not mention {required}")
    board_docker_gate_text = read_text(board_docker_gate)
    for required in (
        "Dockerfile.rtl-board",
        "--privileged",
        "--device",
        "readlink -f",
        "Board UART device does not look like a serial TTY",
        "LNP64_BOARD_EVIDENCE_OUT",
        "LNP64_ICEPROG_DEVICE",
        "/work/build/lnp64-board-ice40-s0-evidence.json",
        "/work/build/lnp64_s0_ice40.bin",
        "run_rtl_board_ice40_s0.sh",
    ):
        require(required in board_docker_gate_text, f"board Docker gate does not mention {required}")
    uart_checker_text = read_text(uart_checker)
    for required in ("serial.Serial", "--expect-hex", "uart byte ok", "Expected UART value is not a byte"):
        require(required in uart_checker_text, f"UART checker does not mention {required}")
    evidence_checker_text = read_text(evidence_checker)
    for required in (
        "lnp64_board_ice40_s0_v1",
        "generated_at_utc",
        "UTC_TIMESTAMP_RE",
        "SERIAL_TTY_RE",
        "bitstream_sha256",
        "sha256_file",
        "bytes_to_hex",
        "captured UART bytes",
        "captured UART bytes do not match UART log contents",
        "preflight_log",
        "program_success_line",
        "program log does not include success line",
        "tool_versions",
        "REQUIRED_TOOLS",
        "board evidence ok",
    ):
        require(required in evidence_checker_text, f"board evidence checker does not mention {required}")

    vertical_slice_tops = manifest.get("vertical_slice_tops")
    require(
        vertical_slice_tops == EXPECTED_VERTICAL_SLICE_TOPS,
        "vertical_slice_tops must list M1-M15 RTL slice tops in order",
    )
    for module in vertical_slice_tops:
        require(re.search(rf"\bmodule\s+{re.escape(module)}\b", combined) or any(
            re.search(rf"\bmodule\s+{re.escape(module)}\b", read_text(path))
            for path in ROOT.glob("rtl/**/*.sv")
        ), f"missing vertical slice module {module}")
        require(
            module in synth_smoke_text,
            f"synthesis smoke gate does not mention vertical slice module {module}",
        )
        require(
            module in vertical_yosys_text,
            f"vertical Yosys gate does not synthesize vertical slice module {module}",
        )

    print("rtl synthesis constraints ok")


if __name__ == "__main__":
    main()

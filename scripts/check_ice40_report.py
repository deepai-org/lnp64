#!/usr/bin/env python3
"""Validate nextpnr-ice40 timing/utilization report for the S0 FPGA smoke."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    parser.add_argument("--min-frequency-mhz", type=float, required=True)
    args = parser.parse_args()

    report = json.loads(Path(args.report).read_text())
    fmax = report.get("fmax")
    require(isinstance(fmax, dict) and fmax, "nextpnr report missing fmax data")

    achieved_values: list[float] = []
    for clock, data in fmax.items():
        require(isinstance(data, dict), f"clock {clock} fmax entry is not an object")
        achieved = data.get("achieved")
        constraint = data.get("constraint")
        require(isinstance(achieved, (int, float)), f"clock {clock} missing achieved fmax")
        require(isinstance(constraint, (int, float)), f"clock {clock} missing timing constraint")
        require(
            achieved >= args.min_frequency_mhz,
            f"clock {clock} achieved {achieved:.3f} MHz below required {args.min_frequency_mhz:.3f} MHz",
        )
        require(
            achieved >= constraint,
            f"clock {clock} achieved {achieved:.3f} MHz below nextpnr constraint {constraint:.3f} MHz",
        )
        achieved_values.append(float(achieved))

    utilization = report.get("utilization")
    require(isinstance(utilization, dict) and utilization, "nextpnr report missing utilization data")
    for resource in ("ICESTORM_LC", "SB_IO"):
        data = utilization.get(resource)
        require(isinstance(data, dict), f"nextpnr report missing {resource} utilization")
        used = data.get("used")
        available = data.get("available")
        require(isinstance(used, int) and used > 0, f"{resource} utilization is not positive")
        require(isinstance(available, int) and available > 0, f"{resource} availability is not positive")
        require(used <= available, f"{resource} utilization exceeds availability: {used}/{available}")

    critical_paths = report.get("critical_paths")
    require(isinstance(critical_paths, list) and critical_paths, "nextpnr report missing critical paths")

    print(f"ice40 timing ok fmax={min(achieved_values):.3f}MHz")


if __name__ == "__main__":
    main()

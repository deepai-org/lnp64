#!/usr/bin/env python3
"""Validate Icestorm icetime summary for the S0 FPGA smoke."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", required=True)
    parser.add_argument("--min-frequency-mhz", type=float, required=True)
    args = parser.parse_args()

    text = Path(args.summary).read_text()
    match = re.search(r"Timing estimate:\s+([0-9.]+)\s+ns\s+\(([0-9.]+)\s+MHz\)", text)
    require(match is not None, "icetime report missing timing estimate")
    estimate_ns = float(match.group(1))
    achieved_mhz = float(match.group(2))
    require(estimate_ns > 0.0, "icetime timing estimate must be positive")
    require(
        achieved_mhz >= args.min_frequency_mhz,
        f"icetime achieved {achieved_mhz:.3f} MHz below required {args.min_frequency_mhz:.3f} MHz",
    )
    require("clock constraint: PASSED" in text, "icetime report does not show a passed clock constraint")

    print(f"icetime timing ok fmax={achieved_mhz:.2f}MHz")


if __name__ == "__main__":
    main()

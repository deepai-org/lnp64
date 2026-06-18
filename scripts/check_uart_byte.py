#!/usr/bin/env python3
"""Capture a UART byte from a serial device during board bring-up."""

from __future__ import annotations

import argparse
import time


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--device", required=True, help="Serial device, for example /dev/ttyUSB1")
    parser.add_argument("--baud", type=int, default=115200, help="UART baud rate")
    parser.add_argument("--expect-hex", required=True, help="Expected byte as hex, for example 53")
    parser.add_argument("--timeout", type=float, default=10.0, help="Capture timeout in seconds")
    parser.add_argument("--output", help="Optional file to write captured bytes")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    try:
        import serial
    except ImportError as exc:
        raise SystemExit("python3-serial is required for UART board validation") from exc

    try:
        expected = int(args.expect_hex, 16)
    except ValueError as exc:
        raise SystemExit(f"Expected UART value is not hex: {args.expect_hex}") from exc
    if expected < 0 or expected > 0xFF:
        raise SystemExit(f"Expected UART value is not a byte: {args.expect_hex}")
    deadline = time.monotonic() + args.timeout
    captured = bytearray()

    with serial.Serial(args.device, args.baud, timeout=0.1) as port:
        port.reset_input_buffer()
        while time.monotonic() < deadline:
            chunk = port.read(64)
            if chunk:
                captured.extend(chunk)
                if expected in chunk:
                    if args.output:
                        with open(args.output, "wb") as out:
                            out.write(captured)
                    print(f"uart byte ok 0x{expected:02x}")
                    return

    if args.output:
        with open(args.output, "wb") as out:
            out.write(captured)
    seen = " ".join(f"0x{byte:02x}" for byte in captured) or "<none>"
    raise SystemExit(f"UART byte 0x{expected:02x} not observed on {args.device}; captured {seen}")


if __name__ == "__main__":
    main()

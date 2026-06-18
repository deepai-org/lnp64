#!/usr/bin/env python3
"""Self-test UART byte capture using a fake pyserial module."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_uart_byte.py"
FAKE_SERIAL = r'''
import os


class Serial:
    def __init__(self, device, baud, timeout=0.1):
        self.device = device
        self.baud = baud
        self.timeout = timeout
        raw = os.environ.get("LNP64_FAKE_UART_BYTES", "")
        self.data = bytes(int(part, 16) for part in raw.split()) if raw else b""
        self.offset = 0

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        return False

    def reset_input_buffer(self):
        pass

    def read(self, size):
        if self.offset >= len(self.data):
            return b""
        chunk = self.data[self.offset:self.offset + size]
        self.offset += len(chunk)
        return chunk
'''


def run_checker(
    tmp: Path,
    fake_bytes: str,
    output: Path,
    expect_hex: str = "53",
) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PYTHONPATH"] = str(tmp)
    env["LNP64_FAKE_UART_BYTES"] = fake_bytes
    result = subprocess.run(
        [
            sys.executable,
            str(CHECKER),
            "--device",
            "/dev/ttyUSB-test",
            "--baud",
            "115200",
            "--expect-hex",
            expect_hex,
            "--timeout",
            "0.01",
            "--output",
            str(output),
        ],
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    result.stdout = (result.stdout or "") + (result.stderr or "")
    return result


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="lnp64-uart-checker-selftest-") as raw_tmp:
        tmp = Path(raw_tmp)
        (tmp / "serial.py").write_text(FAKE_SERIAL, encoding="utf-8")

        positive_output = tmp / "positive.bin"
        positive = run_checker(tmp, "00 52 53 54", positive_output)
        require(positive.returncode == 0, f"expected UART checker success: {positive.stdout}")
        require("uart byte ok 0x53" in positive.stdout, "UART checker did not print success")
        require(positive_output.read_bytes() == bytes([0x00, 0x52, 0x53, 0x54]), "UART output log mismatch")

        prefixed_output = tmp / "prefixed.bin"
        prefixed = run_checker(tmp, "53", prefixed_output, expect_hex="0x53")
        require(prefixed.returncode == 0, f"expected prefixed UART checker success: {prefixed.stdout}")
        require("uart byte ok 0x53" in prefixed.stdout, "prefixed UART checker did not print success")

        negative_output = tmp / "negative.bin"
        negative = run_checker(tmp, "00 52 54", negative_output)
        require(negative.returncode != 0, "expected UART checker failure when byte is absent")
        require(
            "UART byte 0x53 not observed" in negative.stdout,
            f"UART checker failure did not name missing byte: {negative.stdout}",
        )
        require(negative_output.read_bytes() == bytes([0x00, 0x52, 0x54]), "negative UART log mismatch")

        invalid_output = tmp / "invalid.bin"
        invalid = run_checker(tmp, "53", invalid_output, expect_hex="123")
        require(invalid.returncode != 0, "expected UART checker failure for out-of-range byte")
        require(
            "Expected UART value is not a byte" in invalid.stdout,
            f"UART checker failure did not reject out-of-range byte: {invalid.stdout}",
        )

    print("uart byte checker self-test ok")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Convert llvm-objdump instruction bytes into lnp64 flat-exec hex words."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


BYTE_LINE = re.compile(r"^\s*[0-9a-fA-F]+:\s*((?:[0-9a-fA-F]{2}\s+)+)")


def dump_to_words(text: str) -> list[str]:
    bytes_out: list[int] = []
    for line in text.splitlines():
        match = BYTE_LINE.match(line)
        if not match:
            continue
        bytes_out.extend(int(byte, 16) for byte in match.group(1).split())
    if not bytes_out:
        raise SystemExit("llvm objdump did not contain instruction bytes")
    if len(bytes_out) % 4 != 0:
        raise SystemExit(f"llvm objdump byte count is not word-aligned: {len(bytes_out)}")
    words = []
    for idx in range(0, len(bytes_out), 4):
        word = (
            bytes_out[idx]
            | (bytes_out[idx + 1] << 8)
            | (bytes_out[idx + 2] << 16)
            | (bytes_out[idx + 3] << 24)
        )
        words.append(f"{word:08x}")
    return words


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dump", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args()

    words = dump_to_words(args.dump.read_text(encoding="utf-8"))
    output = "\n".join(words) + "\n"
    if args.output:
        args.output.write_text(output, encoding="utf-8")
    else:
        print(output, end="")


if __name__ == "__main__":
    main()

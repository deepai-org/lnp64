#!/usr/bin/env python3
"""Extract one or more named SystemVerilog modules from a source file.

Prints the verbatim text of each requested `module <name> ... endmodule` block
(top-level modules only) in the order requested. Used to isolate a single small
shell module for formal property verification without parsing sibling modules
that the yosys frontend cannot handle.

Usage: extract_sv_module.py <source.sv> <module_name> [<module_name> ...]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


def extract(text: str, name: str) -> str:
    # Match "module <name>" at the start of a line (allow leading whitespace),
    # where <name> is followed by a non-identifier char (space, '#', '(').
    pattern = re.compile(rf"(?m)^[ \t]*module\s+{re.escape(name)}\b")
    m = pattern.search(text)
    if not m:
        raise SystemExit(f"extract_sv_module: module {name} not found")
    start = m.start()
    # Walk tokens from start, tracking module/endmodule nesting (SV modules do
    # not nest in this codebase, but be safe).
    depth = 0
    idx = start
    token = re.compile(r"\bmodule\b|\bendmodule\b")
    while True:
        t = token.search(text, idx)
        if not t:
            raise SystemExit(f"extract_sv_module: no endmodule for {name}")
        if t.group() == "module":
            depth += 1
        else:
            depth -= 1
            if depth == 0:
                return text[start:t.end()]
        idx = t.end()


def main() -> int:
    if len(sys.argv) < 3:
        raise SystemExit("usage: extract_sv_module.py <source.sv> <module> [<module> ...]")
    text = Path(sys.argv[1]).read_text(encoding="utf-8")
    out = [extract(text, name) for name in sys.argv[2:]]
    sys.stdout.write("\n\n".join(out) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

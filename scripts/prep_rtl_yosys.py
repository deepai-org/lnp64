#!/usr/bin/env python3
"""Prepare LNP64 SystemVerilog sources for Yosys package handling."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PKG = ROOT / "rtl/include/lnp64_pkg.sv"


def package_identifiers() -> set[str]:
    pkg_text = PKG.read_text()
    identifiers = set(re.findall(r"}\s+(\w+)\s*;", pkg_text))
    identifiers.update(
        re.findall(r"localparam(?:\s+\w+)?(?:\s+\[[^\]]+\])?\s+(\w+)\s*=", pkg_text)
    )
    identifiers.update(
        re.findall(r"typedef\s+enum\s+logic\s+\[[^\]]+\]\s*\{[^}]*\}\s+(\w+)\s*;", pkg_text, re.S)
    )
    for enum_body in re.findall(r"typedef\s+enum\s+logic\s+\[[^\]]+\]\s*\{([^}]*)\}\s+\w+\s*;", pkg_text, re.S):
        for item in enum_body.split(","):
            name = item.strip().split("=")[0].strip()
            if name:
                identifiers.add(name)
    identifiers.update(re.findall(r"function\s+automatic\s+\w+\s+(\w+)\s*\(", pkg_text))
    return identifiers


def source_list(filelist: Path, extra_sources: list[str]) -> list[Path]:
    sources: list[Path] = []
    for raw in filelist.read_text().splitlines():
        rel = raw.strip()
        if not rel or rel.startswith("#"):
            continue
        sources.append(Path(rel))
    sources.extend(Path(extra) for extra in extra_sources)
    return sources


def rewrite_source(rel: Path, out: Path, identifiers: set[str]) -> None:
    text = (ROOT / rel).read_text()
    if rel != Path("rtl/include/lnp64_pkg.sv"):
        text = text.replace("import lnp64_pkg::*;\n", "")
        for ident in sorted(identifiers, key=len, reverse=True):
            text = re.sub(rf"(?<!::)\b{re.escape(ident)}\b", f"lnp64_pkg::{ident}", text)
        text = rewrite_function_returns(text)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text)


def rewrite_function_returns(text: str) -> str:
    def replace_body(match: re.Match[str]) -> str:
        name = match.group("name")
        body = re.sub(r"\breturn\s+([^;]+);", rf"{name} = \1;", match.group("body"))
        return f"{match.group('header')}{body}{match.group('footer')}"

    return re.sub(
        r"(?P<header>\bfunction\b.*?\b(?P<name>\w+)\s*\([^;]*\)\s*;)(?P<body>.*?)(?P<footer>\bendfunction\b)",
        replace_body,
        text,
        flags=re.S,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--filelist", required=True)
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--sources-out", required=True)
    parser.add_argument("--extra-source", action="append", default=[])
    parser.add_argument("--exclude-prefix", action="append", default=["formal/", "rtl/sim/"])
    args = parser.parse_args()

    out_dir = Path(args.out_dir)
    outputs: list[Path] = []
    identifiers = package_identifiers()

    for rel in source_list(ROOT / args.filelist, args.extra_source):
        rel_text = rel.as_posix()
        if any(rel_text.startswith(prefix) for prefix in args.exclude_prefix):
            continue
        source = ROOT / rel
        if not source.exists():
            raise SystemExit(f"missing RTL source {rel_text}")
        out = out_dir / rel
        rewrite_source(rel, out, identifiers)
        outputs.append(out)

    Path(args.sources_out).write_text("\n".join(str(path) for path in outputs) + "\n")


if __name__ == "__main__":
    main()

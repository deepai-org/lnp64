#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check_rtl_top_level_program_manifest.py >/dev/null

if [[ -z "${LNP64_BIN:-}" ]]; then
  cargo build --quiet
  export LNP64_BIN="$root/target/debug/lnp64"
fi

export LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}"
export LNP64_RTL_BUILD_ROOT="${LNP64_RTL_BUILD_ROOT:-$root/target/rtl-verilator}"

if [[ "$#" -gt 0 ]]; then
  programs=("$@")
else
  mapfile -t programs < <(
    python3 - <<'PY'
import json
from pathlib import Path

manifest = json.loads(Path("tests/rtl/top_level_program_manifest.json").read_text(encoding="utf-8"))
for section in ("flat_hex_programs", "compiler_flat_programs"):
    for entry in manifest[section]:
        if entry["status"] == "active":
            print(entry["source"])
PY
  )
fi

if [[ "${#programs[@]}" -eq 0 ]]; then
  printf '%s\n' "no active top-level RTL programs selected" >&2
  exit 1
fi

first=1
for program in "${programs[@]}"; do
  printf '\n==> top-level RTL program: %s\n' "$program"
  if [[ "$first" -eq 1 ]]; then
    first=0
    bash scripts/run_rtl_top_program_smoke.sh "$program"
  else
    LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}" \
      LNP64_RTL_TOP_PROGRAM_SKIP_BUILD="${LNP64_RTL_TOP_PROGRAM_SKIP_BUILD:-1}" \
      bash scripts/run_rtl_top_program_smoke.sh "$program"
  fi
done

printf '\n%s\n' "rtl top-level program manifest gate ok (${#programs[@]} programs)"

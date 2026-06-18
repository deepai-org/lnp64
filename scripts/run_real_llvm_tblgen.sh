#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tblgen="${LLVM_TBLGEN:-}"
llvm_config="${LLVM_CONFIG:-}"

if [[ -z "$tblgen" ]]; then
  tblgen="$(command -v llvm-tblgen || true)"
fi
if [[ -z "$tblgen" ]]; then
  tblgen="$(find /usr/bin -maxdepth 1 -type f -name 'llvm-tblgen-*' | sort -V | tail -n 1)"
fi
if [[ -z "$tblgen" ]]; then
  printf '%s\n' "llvm-tblgen not found; install LLVM tools or run via Dockerfile.llvm" >&2
  exit 127
fi

if [[ -z "$llvm_config" ]]; then
  llvm_config="$(command -v llvm-config || true)"
fi
if [[ -z "$llvm_config" ]]; then
  llvm_config="$(find /usr/bin -maxdepth 1 -type f -name 'llvm-config-*' | sort -V | tail -n 1)"
fi
if [[ -z "$llvm_config" ]]; then
  printf '%s\n' "llvm-config not found; install llvm-dev or run via Dockerfile.llvm" >&2
  exit 127
fi

include_dir="$("$llvm_config" --includedir)"
out_dir="${LNP64_REAL_LLVM_TBLGEN_OUT:-target/real-llvm-tblgen}"
mkdir -p "$out_dir"

common_args=(
  -I "$include_dir"
  -I "llvm/lib/Target/LNP64"
  "llvm/lib/Target/LNP64/LNP64.td"
)

printf 'using llvm-tblgen: %s\n' "$tblgen"
printf 'using llvm-config: %s\n' "$llvm_config"
printf 'using LLVM include dir: %s\n' "$include_dir"

"$tblgen" -gen-register-info "${common_args[@]}" -o "$out_dir/LNP64GenRegisterInfo.inc"
"$tblgen" -gen-instr-info "${common_args[@]}" -o "$out_dir/LNP64GenInstrInfo.inc"
"$tblgen" -gen-callingconv "${common_args[@]}" -o "$out_dir/LNP64GenCallingConv.inc"
"$tblgen" -gen-subtarget "${common_args[@]}" -o "$out_dir/LNP64GenSubtargetInfo.inc"

printf 'real LLVM TableGen outputs written to %s\n' "$out_dir"

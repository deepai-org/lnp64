#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tag="${LNP64_LLVM_PROJECT_TAG:-llvmorg-14.0.6}"
project_dir="${LNP64_LLVM_PROJECT_DIR:-target/llvm-project-src}"
build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
jobs="${LNP64_LLVM_JOBS:-2}"

rewrite_with_perl() {
  local file="$1"
  shift
  local tmp
  tmp="$(mktemp)"
  cp "$file" "$tmp"
  local expr
  for expr in "$@"; do
    perl -0pi -e "$expr" "$tmp"
  done
  if cmp -s "$file" "$tmp"; then
    rm -f "$tmp"
  else
    mv "$tmp" "$file"
  fi
}

if [[ ! -d "$project_dir/.git" ]]; then
  mkdir -p "$(dirname "$project_dir")"
  git clone \
    --depth 1 \
    --filter=blob:none \
    --sparse \
    --branch "$tag" \
    https://github.com/llvm/llvm-project.git \
    "$project_dir"
  git -C "$project_dir" sparse-checkout set llvm cmake
fi

if [[ ! -f "$project_dir/llvm/CMakeLists.txt" ]]; then
  printf 'LLVM project checkout is missing llvm/CMakeLists.txt: %s\n' "$project_dir" >&2
  exit 1
fi

mkdir -p "$project_dir/llvm/lib/Target/LNP64"
cp -a llvm/lib/Target/LNP64/. "$project_dir/llvm/lib/Target/LNP64/"

llvm_cmake="$project_dir/llvm/CMakeLists.txt"
triple_h="$project_dir/llvm/include/llvm/ADT/Triple.h"
triple_cpp="$project_dir/llvm/lib/Support/Triple.cpp"

if ! grep -q '^  LNP64$' "$llvm_cmake"; then
  perl -0pi -e 's/(^  Lanai\n)/$1  LNP64\n/m' "$llvm_cmake"
fi

rewrite_with_perl "$triple_h" \
  's/^    lnp64,.*\n//mg; s/^    LastArchType = .*/    lnp64,          \/\/ LNP64: LNP64 capability architecture\n    LastArchType = lnp64/m'

rewrite_with_perl "$triple_cpp" \
  's/^  case lnp64:\s+return "lnp64";\n//mg' \
  's/(StringRef Triple::getArchTypeName.*?^  case lanai:\s+return "lanai";\n)/$1  case lnp64:         return "lnp64";\n/ms' \
  's/(StringRef Triple::getArchTypePrefix.*?^  case lanai:\s+return "lanai";\n)/$1  case lnp64:      return "lnp64";\n/ms' \
  's/^  case Triple::lnp64:\n//mg' \
  's/(^  case Triple::lanai:\n)/$1  case Triple::lnp64:\n/m' \
  's/^    \.Case\("lnp64", Triple::lnp64\)\n//mg' \
  's/(^    \.Case\("lanai", Triple::lanai\)\n)/$1    .Case("lnp64", Triple::lnp64)\n/m' \
  's/^  case llvm::Triple::lnp64:\n//mg' \
  's/(^  case llvm::Triple::le64:\n)/$1  case llvm::Triple::lnp64:\n/m'

cmake -S "$project_dir/llvm" -B "$build_dir" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DLLVM_TARGETS_TO_BUILD=LNP64 \
  -DLLVM_INCLUDE_TESTS=OFF \
  -DLLVM_INCLUDE_BENCHMARKS=OFF \
  -DLLVM_INCLUDE_EXAMPLES=OFF \
  -DLLVM_ENABLE_TERMINFO=OFF \
  -DLLVM_ENABLE_ZLIB=OFF \
  -DLLVM_ENABLE_LIBXML2=OFF \
  -DLLVM_ENABLE_LIBEDIT=OFF

ninja -C "$build_dir" -j "$jobs" llc llvm-mc

llc="$build_dir/bin/llc"
llvm_mc="$build_dir/bin/llvm-mc"
"$llc" --version | sed -n '1,12p'

smoke_ir="$(mktemp)"
smoke_asm="$(mktemp)"
smoke_obj="$build_dir/lnp64-smoke.o"
trap 'rm -f "$smoke_ir" "$smoke_asm"' EXIT

cat >"$smoke_ir" <<'IR'
define i64 @main() {
entry:
  ret i64 7
}
IR

"$llc" -mtriple=lnp64-unknown-none -verify-machineinstrs -filetype=null \
  "$smoke_ir" -o /dev/null
"$llc" -mtriple=lnp64-unknown-none "$smoke_ir" -o "$smoke_asm"
grep -q '^li r1, 7$' "$smoke_asm"
grep -q '^ret$' "$smoke_asm"
"$llc" -mtriple=lnp64-unknown-none -filetype=obj "$smoke_ir" -o "$smoke_obj"
test -s "$smoke_obj"
printf 'real LLVM LNP64 llc smoke passed: %s\n' "$smoke_obj"

crt0_obj="$build_dir/crt0-smoke.o"
"$llvm_mc" -triple=lnp64-unknown-none -filetype=obj toolchain/crt0_lnp64.s \
  -o "$crt0_obj"
test -s "$crt0_obj"
printf 'real LLVM LNP64 llvm-mc crt0 smoke passed: %s\n' "$crt0_obj"

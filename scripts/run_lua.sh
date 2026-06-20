#!/usr/bin/env bash
set -euo pipefail

# Minimal upstream-Lua bring-up gate (real_program_ladder rung: minimal_lua).
#
# Builds the standalone Lua 5.4.7 interpreter from the vendored upstream source
# in third_party/lua with the real LNP64 Clang/lld toolchain and the freestanding
# libc shim, then runs `lua -e "print(1+2)"` through `lnp64 run-elf` and checks
# the printed result.
#
# This is intentionally the minimal rung: it opens only the base/string/table/
# coroutine/utf8 libraries (see the generated lnp64 init below) and excludes the
# os/io/math/package libraries, the dynamic C loader, and the luac compiler.
# Broader Lua coverage is tracked in conformance_matrix.md (COMPAT-PKG-001).

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
work_dir="${LNP64_LUA_BUILD_DIR:-target/lnp64-lua-build}"
lua_src="${LNP64_LUA_SRC_DIR:-third_party/lua/src}"
clang="${LNP64_CLANG:-$build_dir/bin/clang}"
lld="${LNP64_LLD:-$build_dir/bin/ld.lld}"
lnp64_bin="${LNP64_BIN:-${CARGO_TARGET_DIR:-target}/debug/lnp64}"
lua_expr="${LNP64_LUA_EXPR:-print(1+2)}"
lua_expect="${LNP64_LUA_EXPECT:-3}"

require_executable() {
  if [[ ! -x "$1" ]]; then
    printf 'missing %s: %s\n' "$2" "$1" >&2
    exit 1
  fi
}

require_file() {
  if [[ ! -s "$1" ]]; then
    printf 'missing %s: %s\n' "$2" "$1" >&2
    exit 1
  fi
}

require_executable "$clang" "LNP64 clang"
require_executable "$lld" "LNP64 ld.lld"

if [[ ! -s "$sysroot/usr/lib/lnp64/crt0.o" ||
      ! -s "$sysroot/usr/lib/lnp64/liblnp64-math-min.o" ||
      ! -s "$sysroot/usr/lib/lnp64/lnp64_static.ld" ]]; then
  bash scripts/package_lnp64_sysroot.sh
fi

if [[ ! -x "$lnp64_bin" ]]; then
  cargo build --quiet --bin lnp64
fi
require_executable "$lnp64_bin" "lnp64 emulator"

lib_dir="$sysroot/usr/lib/lnp64"
linker_script="$lib_dir/lnp64_static.ld"
crt0_obj="$lib_dir/crt0.o"

# Curated libc shim object set for the minimal Lua interpreter.
libc_objs=(
  "$lib_dir/liblnp64-stdio-min.o"
  "$lib_dir/liblnp64-alloc-min.o"
  "$lib_dir/liblnp64-string-min.o"
  "$lib_dir/liblnp64-convert-min.o"
  "$lib_dir/liblnp64-sort-min.o"
  "$lib_dir/liblnp64-startup-min.o"
  "$lib_dir/liblnp64-signal-min.o"
  "$lib_dir/liblnp64-process-min.o"
  "$lib_dir/liblnp64-setjmp-min.o"
  "$lib_dir/liblnp64-math-min.o"
  "$lib_dir/liblnp64-softfloat-min.o"
  "$lib_dir/liblnp64-locale-min.o"
  "$lib_dir/liblnp64-time-min.o"
  "$lib_dir/liblnp64-fd-min.o"
  "$lib_dir/liblnp64-errno-min.o"
)

require_file "$linker_script" "LNP64 linker script"
require_file "$crt0_obj" "LNP64 crt0 object"
for obj in "${libc_objs[@]}"; do
  require_file "$obj" "LNP64 libc shim object"
done

# Lua core + the standard libraries needed for the minimal opened set.
lua_units=(
  lapi lcode lctype ldebug ldo ldump lfunc lgc llex lmem lobject lopcodes
  lparser lstate lstring ltable ltm lundump lvm lzio
  lauxlib lbaselib lcorolib lstrlib ltablib lutf8lib lmathlib
  lua
)

mkdir -p "$work_dir"

# Generated minimal library init: replaces upstream linit.c so only the
# base/string/table/coroutine/utf8 libraries are opened.
init_c="$work_dir/lnp64_lua_minimal_init.c"
cat >"$init_c" <<'C'
#include "lua.h"
#include "lualib.h"
#include "lauxlib.h"

static const luaL_Reg lnp64_minimal_libs[] = {
    {LUA_GNAME, luaopen_base},
    {LUA_TABLIBNAME, luaopen_table},
    {LUA_STRLIBNAME, luaopen_string},
    {LUA_COLIBNAME, luaopen_coroutine},
    {LUA_UTF8LIBNAME, luaopen_utf8},
    {LUA_MATHLIBNAME, luaopen_math},
    {NULL, NULL},
};

LUALIB_API void luaL_openlibs(lua_State *L) {
  const luaL_Reg *lib;
  for (lib = lnp64_minimal_libs; lib->func != NULL; lib++) {
    luaL_requiref(L, lib->name, lib->func, 1);
    lua_pop(L, 1);
  }
}
C

clang_flags=(
  --target=lnp64-unknown-none -ffreestanding -fno-pic
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables
  -Wno-implicit-function-declaration
  -isystem "$sysroot/usr/include" -I toolchain -I "$lua_src"
  -DLUA_USE_C89 -DLUA_USE_JUMPTABLE=0 -O0
)

obj_list=()
compile_unit() {
  local src="$1"
  local obj="$2"
  "$clang" "${clang_flags[@]}" -c "$src" -o "$obj"
  require_file "$obj" "Lua object $(basename "$obj")"
  obj_list+=("$obj")
}

for unit in "${lua_units[@]}"; do
  compile_unit "$lua_src/$unit.c" "$work_dir/$unit.o"
done
compile_unit "$init_c" "$work_dir/lnp64_lua_minimal_init.o"

lua_elf="$work_dir/lnp64-lua-linked.elf"
"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$lua_elf" "$crt0_obj" "${obj_list[@]}" "${libc_objs[@]}"
require_file "$lua_elf" "linked Lua interpreter ELF"
printf 'real LLVM LNP64 lua static link passed: %s\n' "$lua_elf"

"$lnp64_bin" elf-plan "$lua_elf" >/dev/null
printf 'real LLVM LNP64 lua elf-plan passed: %s\n' "$lua_elf"

# run-elf uses the trailing arguments as the full argv, so pass "lua" as argv[0].
run_output="$("$lnp64_bin" run-elf "$lua_elf" lua -e "$lua_expr")"
printf '%s\n' "$run_output"
grep -q "exit=0" <<<"$run_output"
grep -qx "$lua_expect" <<<"$run_output"
printf 'real LLVM LNP64 lua run-elf passed: lua -e "%s" => %s\n' \
  "$lua_expr" "$lua_expect"

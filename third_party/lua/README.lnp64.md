# Lua for LNP64

These files are imported unmodified from upstream Lua 5.4.7:

https://www.lua.org/ftp/lua-5.4.7.tar.gz

Only `src/*.c` and `src/*.h` plus the upstream `README` (which carries the MIT
license text, also reproduced at the end of `src/lua.h`) are vendored. The
upstream `Makefile` and `doc/` are intentionally omitted; LNP64 drives the build
through `scripts/run_lua.sh` with the real Clang/lld toolchain.

## Minimal static build profile

The first ladder rung (`minimal_lua` in
`toolchain/lnp64_real_program_ladder.manifest`) only needs the standalone
interpreter to evaluate `lua -e "print(1+2)"` through Clang/lld/`run-elf`. To
keep that rung honest and small the LNP64 profile:

- Compiles against the freestanding LNP64 libc shim under `toolchain/`, not a
  host libc.
- Defines `LUA_USE_C89` and leaves `LUA_USE_POSIX`/`LUA_USE_DLOPEN` off, so the
  interpreter avoids `dlopen`, `popen`, and POSIX-only goodies.
- Drops the dynamic `package`/`require` C-loader path (`loadlib.c`) and the
  `os`/`io` libraries from the opened standard-library set; the minimal init in
  `scripts/run_lua.sh` opens only the base, string, table, math, coroutine, and
  utf8 libraries that the first rung needs.
- Relies on a minimal libm shim (`toolchain/include/math.h`,
  `toolchain/liblnp64_math_min.c`) for the `floor`/`fmod`/`pow` references the
  core VM (`lvm.c`) and `lmathlib.c` emit.

Broader Lua surface (full `os`/`io`, `require`, the `luac` compiler, and running
real `.lua` scripts) is tracked as remaining work in `conformance_matrix.md`
under `COMPAT-PKG-001`.

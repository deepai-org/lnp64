# System Software Compatibility Roadmap

High-level plan for getting from the current state to broader real-world system
software compatibility (Lua → SQLite → Redis and beyond) on LNP64.

## Status recap

Upstream Lua 5.4.7 now runs under the emulator: `scripts/run_lua.sh` builds the
real interpreter with Clang/lld and `lua -e "print(1+2)" => 3` passes through
`run-elf`. Getting there required fixing a generic `ISD::SELECT` backend codegen
crash, adding a double soft-float runtime, and filling several libc/header gaps
— all reusable by every C program, not Lua-specific. The `minimal_lua` ladder
rung (`toolchain/lnp64_real_program_ladder.manifest`) is flipped to `tested`;
the conformance matrix and the ladder manifest test are updated.

Richer Lua expressions confirm integer arithmetic, strings, tables, `//`, and
`%` work. Known immediate gap: `print(10/4)` and `print(2^10)` print the literal
`%.14g` — the `snprintf` shim does not implement `%g`/`%f` float conversions
yet (float math itself runs; only formatting is missing). The `math` library is
absent by design for the minimal rung.

## The governing method

The ladder in `toolchain/lnp64_real_program_ladder.manifest` is the spine. Each
real program is chosen to expose **generic** runtime/compiler gaps. The rule
(already paying off): **fix every gap in the lowest correct layer** — LLVM
backend → soft-float/libc runtime → loader/personality → *never* a per-app
shim. Lua alone forced a backend fix + a soft-float library + nine libc/header
additions that SQLite and Redis will all reuse.

## Cross-cutting foundations (unblock everything downstream)

These are not tied to one app; they gate the whole ladder:

1. **printf/scanf float formatting** (`%g`/`%f`/`%e`, and `strtod` accuracy).
   Surfaced via `print(10/4)`. Needed by Lua output, SQLite (`printf`-heavy),
   and Redis logging. Lowers onto the soft-float lib already in place.
2. **Optimization levels.** Everything currently builds at `-O0`. There is a
   known `-O1` Greedy-RegAlloc crash (hit on `lua_isnumber`). Fixing it (and
   validating `-O2`) matters for Redis-sized code and realistic performance.
   Track as a backend task parallel to the ladder.
3. **Soft-float completeness & provenance.** The hand-written
   `toolchain/liblnp64_softfloat_min.c` is host-validated for the double ops Lua
   emits. Decide whether to keep extending it or vendor compiler-rt `builtins`
   (the LLVM sparse-checkout currently omits compiler-rt) for the long tail
   (`__floatti`, `__powidf2`, fenv, etc.).
4. **Loader maturity.** `run_lua.sh` uses the *flat* exec path. SQLite/Redis
   will want the real ELF/VMA/MMU software loader (`elf-plan` exists; the flat
   shortcut does not materialize full segments). Maturing the loader is on the
   critical path for anything with real `mmap`/segmented layouts.

## Rung-by-rung

### A. Finish Lua breadth (small, immediate, proves the runtime)

- Float formatting (foundation #1), then open `math`/`io`/`os` libraries and run
  real `.lua` scripts from a file. Each opened library is a libc surface test
  (`io` → file streams, `os` → time/getenv/exit). Converts `COMPAT-PKG-001`
  from "minimal" to "broad."

### B. SQLite — in-memory, then file-backed (`sqlite_memory_file`, `blocked`)

- Vendor the upstream amalgamation (single large C file — stresses the compiler
  at scale and `-O1`).
- Drive `:memory:` first (no FS dependency) → exercises realloc-heavy
  allocation, 64-bit arithmetic, varargs formatting.
- Then file-backed: real file semantics, `mmap` or a documented fallback,
  locking policy, durability (`fsync`-shaped). This is where loader/VFS maturity
  (foundation #4) is tested. SQLite's built-in `sqlite3_exec`
  "CREATE/INSERT/SELECT" makes a clean gate.

### C. NetBSD/POSIX personality closure (`netbsd_posix_personality_closure`)

- The shared investment, deliberately *before* Redis: close fork/waitpid/
  SIGCHLD, signals, timers, sockets, errno fidelity, and the libpthread/libm
  runtime so Redis is not an app-specific hack. Much scaffolding exists
  (`scripts/run_netbsd_personality_system.sh`, kqueue/poll/signal work). The job
  is closing the gaps SQLite and the daemons reveal.

### D. Tiny network daemons (`tiny_network_daemons`: netcat/httpd)

- Prove sockets bind/listen/accept/connect, nonblocking + readiness
  (`poll`/`select`/`epoll`/`kqueue`), `EINTR`/`EAGAIN`/`ECONNRESET`, and clean
  signal shutdown — under an unattended loopback harness. These are exactly
  Redis's event-loop primitives, de-risked on smaller programs first.

### E. Redis (`redis_configured_build` → `redis_single_client` → `redis_persistence_fork`)

- Only after C/D. Static build, no TLS/modules; then single-client
  `PING/SET/GET/DEL` over the event loop; then persistence + background-save
  fork with temp-rename/`fsync`/`waitpid`/`SIGCHLD`. Redis mostly *composes*
  foundations from A–D rather than introducing new primitives — which is the
  whole point of sequencing it last.

## How each rung stays honest

Every rung gets: vendored upstream source under `third_party/`, a
`scripts/run_<pkg>.sh` real-Clang/lld/`run-elf` gate, a ladder-manifest row
(`planned` → `tested`), and a conformance-matrix update. Gaps found become
entries against the cross-cutting foundations, not the app.

## Suggested immediate order

1. ✅ **DONE** Float formatting `%g`/`%f` (unblocks Lua float output + SQLite/Redis
   logging) — smallest, highest leverage. Implemented %f (fixed), %g (general),
   %e (scientific) format specifiers with precision parsing in snprintf. Lua
   `print(10/4)` => `2.5000000000000e+00` now works.
2. Open Lua `math`/`io`/`os` + run a `.lua` script file.
3. Start SQLite `:memory:`, surfacing the next batch of generic gaps (likely
   `-O1`/regalloc, more `printf`, larger frames).
4. Loop foundations (`-O1` regalloc, loader/VFS) as SQLite demands them.

# libc-test on LNP64

This directory vendors a narrow subset of libc-test from the musl project,
using upstream commit `b95fe84efa38a146ad32ae02cac224e928926810` as fetched
from `https://repo.or.cz/libc-test.git`.

The checked gate currently runs these functional and regression tests. Most are
upstream libc-test files; bounded or locally guarded files are called out below.

- `functional/argv.c`
- `functional/access_bounded.c`
- `functional/basename.c`
- `functional/clock_gettime.c`
- `functional/ctype_bounded.c`
- `functional/dirname.c`
- `functional/env.c`
- `functional/fdopen.c`
- `functional/fcntl_basic_bounded.c`
- `functional/fcntl.c`
- `functional/pthread_tsd.c`
- `functional/qsort_bounded.c`
- `functional/random.c`
- `functional/search_insque.c`
- `functional/search_lsearch.c`
- `functional/sem_init.c`
- `functional/stat.c`
- `functional/string.c`
- `functional/string_memcpy_bounded.c`
- `functional/string_memmem.c`
- `functional/string_memmove_bounded.c`
- `functional/string_strchr.c`
- `functional/string_strcspn.c`
- `functional/string_strstr.c`
- `functional/strtol.c`
- `functional/udiv.c`
- `functional/ungetc.c`
- `functional/utime.c`
- `regression/fgets-eof.c`
- `regression/malloc-0.c`

The local `functional/test.h` and `functional/print.c` files provide a minimal
LNP64-compatible test harness. They preserve the libc-test convention that tests
return zero on success and set `t_status` after `t_error` on failure, but avoid
the broader upstream harness dependencies that are outside this focused gate.
`qsort_bounded.c` follows the upstream qsort coverage shape but reduces the
large uint64 cases because the current compiler qsort helper is O(n^2); the full
upstream qsort case is a remaining performance gate, not a known correctness
failure. `udiv.c` keeps the checked unsigned division rows but skips a local
zero-divisor table row because that row is undefined behavior in C.
`ungetc.c` is the upstream file and covers descriptor-backed one-byte
pushback, scanset mismatch preservation, `ftell` positioning, and `fread`
consumption of an ungot byte.
`stat.c` is the upstream file and covers directory/device predicates, timestamp
fields, file size, and guest UID/GID ownership in exported `struct stat`
records.
`utime.c` is the upstream file and covers `futimens`/`utimensat` timestamp
pairs, `UTIME_NOW`, `UTIME_OMIT`, invalid file descriptors, and nested
timestamp fields in `struct stat`.
`access_bounded.c` is a local bounded file covering metadata-backed `access`
existence checks, supported mode constants, missing-path `ENOENT`, and
invalid-mode `EINVAL`.
`fcntl.c` is the upstream file and covers whole-file advisory lock rejection
and `F_GETLK` owner reporting across `fork`.
`fcntl_basic_bounded.c` is a local bounded file covering descriptor flag queries,
zero-valued control calls, and unsupported-command `EINVAL` without depending on
cross-process advisory locks.
`sem_init.c` is the upstream file and covers unnamed semaphore init/wait/post,
try/timed wait errno behavior, and pthread start routines that return normally.
`pthread_tsd.c` is the upstream file and covers thread-specific storage
isolation, destructor execution on thread exit, and key deletion.
`regression/fgets-eof.c` is the upstream file with only the harness include path
adjusted; it covers `fmemopen` reads through `fgets` and EOF buffer preservation.
`regression/malloc-0.c` is the upstream file with only the harness include path
adjusted; it covers non-null, unique `malloc(0)` allocations.
`functional/ctype_bounded.c` is a local bounded file covering the supported
ASCII ctype predicates and `tolower`/`toupper` mappings without locale tables.
`functional/string_memcpy_bounded.c` is a local bounded file covering direct and
function-pointer `memcpy`/`memset`, offset destinations, high-byte `memset`
values, and zero-length no-write behavior without the full upstream exhaustive
alignment loops.
`functional/string_memmove_bounded.c` is a local bounded file covering direct
and function-pointer `memmove` with non-overlap, destination-after-source
overlap, destination-before-source overlap, and zero-length moves.

## Real Clang replacement status

The real LLVM/Clang/lld `run-elf` gate now covers former toy-only `pthread_tsd.c`
and `sem_init.c` cases, plus bounded `access_bounded.c` metadata probing and
`fcntl_basic_bounded.c` descriptor-flag coverage. Upstream `fcntl.c` remains
toy-only until POSIX `fork()` lowers through the formal CLONE-profile
compatibility layer and `waitpid()` lowers through the event/waitable path. Do
not add new Rust toy compiler fcntl/fork language features to close that gap.

Do not add new Rust toy compiler fcntl/fork language features.

Run the real Clang/lld replacement gate with:

```sh
bash scripts/run_libc_test.sh
```

Run the legacy toy-bootstrap subset explicitly with:

```sh
bash scripts/run_libc_test.sh --backend toy --loader asm
```

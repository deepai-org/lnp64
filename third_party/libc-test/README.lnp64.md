# libc-test on LNP64

This directory vendors a narrow subset of libc-test from the musl project,
using upstream commit `b95fe84efa38a146ad32ae02cac224e928926810` as fetched
from `https://repo.or.cz/libc-test.git`.

The checked gate currently runs these functional tests. Most are upstream
libc-test files; bounded or locally guarded files are called out below.

- `functional/argv.c`
- `functional/basename.c`
- `functional/clock_gettime.c`
- `functional/dirname.c`
- `functional/env.c`
- `functional/fdopen.c`
- `functional/qsort_bounded.c`
- `functional/random.c`
- `functional/search_insque.c`
- `functional/search_lsearch.c`
- `functional/string.c`
- `functional/string_memmem.c`
- `functional/string_strchr.c`
- `functional/string_strcspn.c`
- `functional/string_strstr.c`
- `functional/strtol.c`
- `functional/udiv.c`
- `functional/ungetc.c`

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

Run the subset with:

```sh
bash scripts/run_libc_test.sh
```

# libc-test on LNP64

This directory vendors a narrow subset of libc-test from the musl project,
using upstream commit `b95fe84efa38a146ad32ae02cac224e928926810` as fetched
from `https://repo.or.cz/libc-test.git`.

The checked gate currently runs these upstream functional tests:

- `functional/argv.c`
- `functional/basename.c`
- `functional/clock_gettime.c`
- `functional/dirname.c`
- `functional/env.c`
- `functional/fdopen.c`
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

The local `functional/test.h` and `functional/print.c` files provide a minimal
LNP64-compatible test harness. They preserve the libc-test convention that tests
return zero on success and set `t_status` after `t_error` on failure, but avoid
the broader upstream harness dependencies that are outside this focused gate.

Run the subset with:

```sh
bash scripts/run_libc_test.sh
```

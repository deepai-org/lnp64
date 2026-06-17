# natsort LNP64 smoke

This directory vendors the `strnatcmp.c`/`strnatcmp.h` core from
`sourcefrog/natsort` at commit `cdd8df9`.

The LNP64 gate compiles the upstream natural-order string comparator unchanged
with `smoke.c` and checks numeric ordering, fractional leading-zero ordering,
and case-folded comparisons.

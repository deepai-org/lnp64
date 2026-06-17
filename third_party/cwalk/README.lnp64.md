# cwalk on LNP64

This directory vendors cwalk from upstream commit
`e98d23f68807208952c179b49e4fd1813f31298d`.

The checked gate builds the upstream `include/cwalk.h` and `src/cwalk.c` with a
small smoke program that exercises Unix and Windows path style selection,
absolute/relative predicates, basename/dirname output parameters, segment
iteration, extension replacement, normalization, path joining, absolute and
relative path generation, and intersection detection.

Run it with:

```sh
bash scripts/run_cwalk.sh
```

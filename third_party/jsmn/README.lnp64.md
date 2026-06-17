# jsmn on LNP64

This directory vendors the upstream `jsmn` single-header JSON parser and its
example/test sources.

The LNP64 gate compiles the upstream header with:

- `example/simple.c`
- `test/tests.c`

Run it with:

```sh
bash scripts/run_jsmn.sh
```

The gate is intentionally separate from `scripts/run_sbase.sh` so jsmn remains
visible as its own real C package target in the compatibility matrix.

#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --quiet --)
tests=(
  argv
  basename
  clock_gettime
  dirname
  env
  fdopen
  qsort_bounded
  random
  search_insque
  search_lsearch
  string
  string_memmem
  string_strchr
  string_strcspn
  string_strstr
  strtol
  udiv
  ungetc
)

for test_name in "${tests[@]}"; do
  asm="/tmp/libc_test_${test_name}.s"
  "${lnp64[@]}" cc \
    "third_party/libc-test/functional/${test_name}.c" \
    third_party/libc-test/functional/print.c \
    -o "$asm"
  out=$("${lnp64[@]}" run "$asm" -- "$test_name")
  test "$out" = ""
done

printf '%s\n' "libc-test subset ok"

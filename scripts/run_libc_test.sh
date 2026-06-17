#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --quiet --)
tests=(
  functional/argv
  functional/basename
  functional/clock_gettime
  functional/dirname
  functional/env
  functional/fdopen
  functional/fcntl
  functional/qsort_bounded
  functional/pthread_tsd
  functional/random
  functional/search_insque
  functional/search_lsearch
  functional/sem_init
  functional/stat
  functional/string
  functional/string_memcpy_bounded
  functional/string_memmem
  functional/string_memmove_bounded
  functional/string_strchr
  functional/string_strcspn
  functional/string_strstr
  functional/strtol
  functional/udiv
  functional/ungetc
  functional/utime
  regression/fgets-eof
  regression/malloc-0
)

for test_path in "${tests[@]}"; do
  test_name="${test_path##*/}"
  asm="/tmp/libc_test_${test_name}.s"
  "${lnp64[@]}" cc \
    "third_party/libc-test/${test_path}.c" \
    third_party/libc-test/functional/print.c \
    -o "$asm"
  out=$("${lnp64[@]}" run "$asm" -- "$test_name")
  test "$out" = ""
done

printf '%s\n' "libc-test subset ok"

#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

backend="llvm"
loader="exec-plan"

usage() {
  cat <<'USAGE'
usage: scripts/run_libc_test.sh [--backend llvm] [--loader asm|exec-plan] [--legacy-toy]

The default llvm/exec-plan backend dispatches to the real Clang/lld/run-elf
gate and must not route through the Rust bootstrap compiler. Use --legacy-toy
only for the legacy libc-test subset that still runs through lnp64 cc
--toy-bootstrap.
USAGE
}

while (($#)); do
  case "$1" in
    --backend)
      backend="${2:-}"
      if [[ -z "$backend" ]]; then
        printf '%s\n' "missing value for --backend" >&2
        usage >&2
        exit 2
      fi
      if [[ "$backend" == "toy" ]]; then
        printf '%s\n' "toy backend is legacy-only; use --legacy-toy" >&2
        exit 2
      fi
      shift 2
      ;;
    --loader)
      loader="${2:-}"
      if [[ -z "$loader" ]]; then
        printf '%s\n' "missing value for --loader" >&2
        usage >&2
        exit 2
      fi
      shift 2
      ;;
    --legacy-toy)
      backend="toy"
      loader="asm"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$backend" in
  toy)
    if [[ "$loader" != "asm" ]]; then
      printf '%s\n' "toy backend only supports --loader asm" >&2
      exit 2
    fi
    ;;
  llvm)
    if [[ "$loader" != "exec-plan" ]]; then
      printf '%s\n' "llvm backend requires --loader exec-plan" >&2
      exit 2
    fi
    exec bash scripts/run_real_llvm_lnp64_docker.sh
    ;;
  *)
    printf 'unknown backend: %s\n' "$backend" >&2
    usage >&2
    exit 2
    ;;
esac

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --quiet --)
fi
tests=(
  functional/argv
  functional/basename
  functional/clock_gettime
  functional/ctype_bounded
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
  "${lnp64[@]}" cc --toy-bootstrap \
    "third_party/libc-test/${test_path}.c" \
    third_party/libc-test/functional/print.c \
    -o "$asm"
  out=$("${lnp64[@]}" run "$asm" -- "$test_name")
  test "$out" = ""
done

printf '%s\n' "legacy toy-bootstrap libc-test subset ok"

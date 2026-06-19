#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${LNP64_LLVM_DOCKER_IMAGE:-lnp64-real-llvm:bookworm}"
uid="$(id -u)"
gid="$(id -g)"

if [[ "${LNP64_LLVM_DOCKER_SKIP_BUILD:-0}" != "1" ]]; then
  docker build -f Dockerfile.llvm -t "$image" .
fi
docker run --rm \
  --user "$uid:$gid" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_real_llvm_lnp64.sh

if [[ "${LNP64_LLVM_DOCKER_SKIP_RUN_ELF:-0}" == "1" ]]; then
  printf 'real LLVM LNP64 run-elf execution skipped by LNP64_LLVM_DOCKER_SKIP_RUN_ELF=1\n'
  exit 0
fi

cargo build --quiet --bin lnp64
lnp64_bin="${CARGO_TARGET_DIR:-target}/debug/lnp64"
if [[ ! -x "$lnp64_bin" ]]; then
  printf 'missing built lnp64 binary: %s\n' "$lnp64_bin" >&2
  exit 1
fi

run_elf_probe() {
  local linked_probe="$1"
  shift
  local run_args=()
  local has_arg_marker=0
  local item
  for item in "$@"; do
    if [[ "$item" == "--expect" ]]; then
      has_arg_marker=1
      break
    fi
  done
  if [[ "$has_arg_marker" == "1" ]]; then
    while [[ "$#" -gt 0 && "$1" != "--expect" ]]; do
      run_args+=("$1")
      shift
    done
    shift
  fi
  "$lnp64_bin" elf-plan "$linked_probe" >/dev/null
  local run_elf_output
  run_elf_output="$("$lnp64_bin" run-elf "$linked_probe" "${run_args[@]}")"
  grep -q 'exit=0' <<<"$run_elf_output"
  local expected
  for expected in "$@"; do
    grep -q "$expected" <<<"$run_elf_output"
  done
}

run_elf_report() {
  local message="$1"
  local linked_probe="$2"
  shift 2
  run_elf_probe "$linked_probe" "$@"
  printf '%s: %s\n' "$message" "$linked_probe"
}

for demo in hello factorial allocator fibonacci pcr cat json-parser rot13 producer-consumer parallel-hash sqlite-lite ping-pong; do
  linked_probe="target/llvm-lnp64-build/lnp64-$demo-clang-linked.elf"
  case "$demo" in
    hello) run_elf_probe "$linked_probe" 'hello from LNP64' ;;
    factorial) run_elf_probe "$linked_probe" 'factorial ok' ;;
    allocator) run_elf_probe "$linked_probe" 'alloc ok' ;;
    fibonacci) run_elf_probe "$linked_probe" 'fibonacci ok' ;;
    pcr) run_elf_probe "$linked_probe" 'pcr ok' ;;
    cat) run_elf_probe "$linked_probe" 'cat ok' ;;
    json-parser) run_elf_probe "$linked_probe" 'json parser ok' ;;
    rot13) run_elf_probe "$linked_probe" 'rot13 ok' ;;
    producer-consumer) run_elf_probe "$linked_probe" 'producer consumer ok' ;;
    parallel-hash) run_elf_probe "$linked_probe" 'parallel hash ok' ;;
    sqlite-lite) run_elf_probe "$linked_probe" 'sqlite lite ok' ;;
    ping-pong) run_elf_probe "$linked_probe" 'ping pong ok' ;;
  esac
done
printf 'real LLVM LNP64 run-elf clang demo execution passed: %s\n' \
  "target/llvm-lnp64-build/lnp64-{hello,factorial,allocator,fibonacci,pcr,cat,json-parser,rot13,producer-consumer,parallel-hash,sqlite-lite,ping-pong}-clang-linked.elf"

run_elf_report "real LLVM LNP64 run-elf native heap execution passed" \
  target/llvm-lnp64-build/lnp64-native-heap-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic push execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-push-linked.elf \
  'intrinsic push ok'
run_elf_report "real LLVM LNP64 run-elf intrinsic await execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-await-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic call execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-call-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic gate return execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-gate-return-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic control execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-control-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic capability control execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-cap-control-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic mmap execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-mmap-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic GET_PCR execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-get-pcr-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic OPEN_AT execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-openat-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic CLONE execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-clone-linked.elf
run_elf_report "real LLVM LNP64 run-elf intrinsic AMO execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-amo-linked.elf
run_elf_report "real LLVM LNP64 run-elf C11 atomic execution passed" \
  target/llvm-lnp64-build/lnp64-c11-atomic-linked.elf
run_elf_report "real LLVM LNP64 run-elf inline asm execution passed" \
  target/llvm-lnp64-build/lnp64-inline-asm-linked.elf
run_elf_report "real LLVM LNP64 run-elf exit execution passed" \
  target/llvm-lnp64-build/lnp64-exit-linked.elf
run_elf_report "real LLVM LNP64 run-elf errno execution passed" \
  target/llvm-lnp64-build/lnp64-errno-linked.elf
run_elf_report "real LLVM LNP64 run-elf argc execution passed" \
  target/llvm-lnp64-build/lnp64-argc-linked.elf
run_elf_report "real LLVM LNP64 run-elf startup argv/envp execution passed" \
  target/llvm-lnp64-build/lnp64-startup-linked.elf
run_elf_report "real LLVM LNP64 run-elf getauxval execution passed" \
  target/llvm-lnp64-build/lnp64-getauxval-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test argv execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-argv-linked.elf \
  lnp64-argv --expect
run_elf_report "real LLVM LNP64 run-elf libc-test env execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-env-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test random execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-random-linked.elf
run_elf_report "real LLVM LNP64 run-elf scalar arithmetic execution passed" \
  target/llvm-lnp64-build/lnp64-scalar-arith-linked.elf
run_elf_report "real LLVM LNP64 run-elf high-multiply execution passed" \
  target/llvm-lnp64-build/lnp64-high-mul-linked.elf
run_elf_report "real LLVM LNP64 run-elf scalar extension execution passed" \
  target/llvm-lnp64-build/lnp64-scalar-extend-linked.elf
run_elf_report "real LLVM LNP64 run-elf bit-manip execution passed" \
  target/llvm-lnp64-build/lnp64-bitmanip-linked.elf
run_elf_report "real LLVM LNP64 run-elf csel execution passed" \
  target/llvm-lnp64-build/lnp64-csel-linked.elf
run_elf_report "real LLVM LNP64 run-elf call-clobber execution passed" \
  target/llvm-lnp64-build/lnp64-call-clobber-linked.elf
run_elf_report "real LLVM LNP64 run-elf comparison execution passed" \
  target/llvm-lnp64-build/lnp64-compare-linked.elf
run_elf_report "real LLVM LNP64 run-elf unsigned comparison execution passed" \
  target/llvm-lnp64-build/lnp64-unsigned-compare-linked.elf
run_elf_report "real LLVM LNP64 run-elf signed-load execution passed" \
  target/llvm-lnp64-build/lnp64-signed-load-linked.elf
run_elf_report "real LLVM LNP64 run-elf wide-constant execution passed" \
  target/llvm-lnp64-build/lnp64-wide-const-linked.elf
run_elf_report "real LLVM LNP64 run-elf stack aggregate execution passed" \
  target/llvm-lnp64-build/lnp64-stack-aggregate-linked.elf
run_elf_report "real LLVM LNP64 run-elf stack-argument execution passed" \
  target/llvm-lnp64-build/lnp64-stack-args-linked.elf
run_elf_report "real LLVM LNP64 run-elf minilibc string execution passed" \
  target/llvm-lnp64-build/lnp64-libc-string-linked.elf
run_elf_report "real LLVM LNP64 run-elf numeric conversion execution passed" \
  target/llvm-lnp64-build/lnp64-convert-linked.elf
run_elf_report "real LLVM LNP64 run-elf path helper execution passed" \
  target/llvm-lnp64-build/lnp64-path-linked.elf
run_elf_report "real LLVM LNP64 run-elf search helper execution passed" \
  target/llvm-lnp64-build/lnp64-search-linked.elf
run_elf_report "real LLVM LNP64 run-elf sort helper execution passed" \
  target/llvm-lnp64-build/lnp64-sort-linked.elf
run_elf_report "real LLVM LNP64 run-elf zlib package execution passed" \
  target/llvm-lnp64-build/lnp64-zlib-linked.elf
run_elf_report "real LLVM LNP64 run-elf natsort package execution passed" \
  target/llvm-lnp64-build/lnp64-natsort-linked.elf
run_elf_report "real LLVM LNP64 run-elf jsmn package execution passed" \
  target/llvm-lnp64-build/lnp64-jsmn-linked.elf
run_elf_report "real LLVM LNP64 run-elf inih package execution passed" \
  target/llvm-lnp64-build/lnp64-inih-linked.elf
run_elf_report "real LLVM LNP64 run-elf cwalk package execution passed" \
  target/llvm-lnp64-build/lnp64-cwalk-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test ctype_bounded execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-ctype-bounded-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test string execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-string-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test string_memcpy_bounded execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-string-memcpy-bounded-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test string_memmove_bounded execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-string-memmove-bounded-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test string_memmem execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-string-memmem-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test string_strchr execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-string-strchr-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test string_strcspn execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-string-strcspn-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test string_strstr execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-string-strstr-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test udiv execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-udiv-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test basename execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-basename-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test dirname execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-dirname-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test strtol execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-strtol-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test clock_gettime execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-clock-gettime-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test stat execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-stat-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test utime execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-utime-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test ungetc execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-ungetc-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test fdopen execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-fdopen-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test pthread_tsd execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-pthread-tsd-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test qsort_bounded execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-qsort-bounded-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test search_insque execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-search-insque-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test search_lsearch execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-search-lsearch-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test malloc-0 execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-malloc-0-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test fgets-eof execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-fgets-eof-linked.elf
run_elf_report "real LLVM LNP64 run-elf calloc execution passed" \
  target/llvm-lnp64-build/lnp64-calloc-linked.elf
run_elf_report "real LLVM LNP64 run-elf realloc execution passed" \
  target/llvm-lnp64-build/lnp64-realloc-linked.elf
run_elf_report "real LLVM LNP64 run-elf read execution passed" \
  target/llvm-lnp64-build/lnp64-read-linked.elf
run_elf_report "real LLVM LNP64 run-elf write execution passed" \
  target/llvm-lnp64-build/lnp64-write-linked.elf \
  'fd write ok'
run_elf_report "real LLVM LNP64 run-elf metadata libc execution passed" \
  target/llvm-lnp64-build/lnp64-meta-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf mmap libc execution passed" \
  target/llvm-lnp64-build/lnp64-mmap-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf futex libc execution passed" \
  target/llvm-lnp64-build/lnp64-futex-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf poll/select/epoll/kqueue libc execution passed" \
  target/llvm-lnp64-build/lnp64-poll-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf signal libc execution passed" \
  target/llvm-lnp64-build/lnp64-signal-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf socket libc execution passed" \
  target/llvm-lnp64-build/lnp64-socket-libc-linked.elf
run_elf_report "real LLVM LNP64 run-elf sbase echo execution passed" \
  target/llvm-lnp64-build/lnp64-sbase-echo-linked.elf \
  echo hello clang --expect 'hello clang'
run_elf_report "real LLVM LNP64 run-elf sbase basename execution passed" \
  target/llvm-lnp64-build/lnp64-sbase-basename-linked.elf \
  basename /usr/local/bin/clang --expect '^clang$'
run_elf_report "real LLVM LNP64 run-elf sbase dirname execution passed" \
  target/llvm-lnp64-build/lnp64-sbase-dirname-linked.elf \
  dirname /usr/local/bin/clang --expect '^/usr/local/bin$'
sbase_fixture_root="target/llvm-lnp64-build/sbase-fixture-root"
mkdir -p "$sbase_fixture_root/input"
printf 'cat via clang\n' >"$sbase_fixture_root/input/cat.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-cat-linked.elf \
  >/dev/null
sbase_cat_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-cat-linked.elf cat input/cat.txt)"
grep -q '^cat via clang$' <<<"$sbase_cat_output"
grep -q 'exit=0' <<<"$sbase_cat_output"
printf 'real LLVM LNP64 run-elf sbase cat execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-cat-linked.elf
run_elf_report "real LLVM LNP64 run-elf indirect call execution passed" \
  target/llvm-lnp64-build/lnp64-indirect-call-linked.elf

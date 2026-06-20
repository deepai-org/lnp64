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
  -e LNP64_LLVM_GATE="${LNP64_LLVM_GATE:-full}" \
  -e LNP64_LLVM_JOBS="${LNP64_LLVM_JOBS:-}" \
  -v "$root:/work" \
  -w /work \
  "$image" \
  bash scripts/run_real_llvm_lnp64.sh

if [[ "${LNP64_LLVM_GATE:-full}" != "full" ]]; then
  printf 'real LLVM LNP64 run-elf execution skipped by LNP64_LLVM_GATE=%s\n' \
    "${LNP64_LLVM_GATE:-full}"
  exit 0
fi

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
run_elf_report "real LLVM LNP64 run-elf intrinsic SET_PCR execution passed" \
  target/llvm-lnp64-build/lnp64-intrinsic-set-pcr-linked.elf
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
run_elf_report "real LLVM LNP64 run-elf libc-test access_bounded execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-access-bounded-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test stat execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-stat-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test utime execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-utime-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test ungetc execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-ungetc-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test fdopen execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-fdopen-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test fcntl_basic_bounded execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-fcntl-basic-bounded-linked.elf
tmproot="${TMPDIR:-/tmp}/lnp64-libc-test-fcntl-root"
rm -rf "$tmproot"
mkdir -p "$tmproot/tmp"
run_elf_report "real LLVM LNP64 run-elf libc-test fcntl execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-fcntl-linked.elf \
  --namespace-root "$tmproot" --expect
run_elf_report "real LLVM LNP64 run-elf libc-test pthread_tsd execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-pthread-tsd-linked.elf
run_elf_report "real LLVM LNP64 run-elf libc-test sem_init execution passed" \
  target/llvm-lnp64-build/lnp64-libc-test-sem-init-linked.elf
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
run_elf_report "real LLVM LNP64 run-elf NetBSD personality clang smoke passed" \
  target/llvm-lnp64-build/lnp64-netbsd-personality-clang-linked.elf \
  'netbsd clang personality init' \
  'netbsd clang personality shell' \
  'netbsd clang personality smoke ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD loader target child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-loader-target-linked.elf \
  'loader_target ok'
netbsd_elf_exec_fixture_root="target/llvm-lnp64-build/netbsd-elf-exec-fixture-root"
rm -rf "$netbsd_elf_exec_fixture_root"
mkdir -p "$netbsd_elf_exec_fixture_root/bin"
cp target/llvm-lnp64-build/lnp64-netbsd-loader-target-linked.elf \
  "$netbsd_elf_exec_fixture_root/bin/loader_target.elf"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-netbsd-elf-exec-test-linked.elf \
  >/dev/null
netbsd_elf_exec_output="$("$lnp64_bin" run-elf --namespace-root "$netbsd_elf_exec_fixture_root" \
  target/llvm-lnp64-build/lnp64-netbsd-elf-exec-test-linked.elf)"
grep -q 'loader_target ok' <<<"$netbsd_elf_exec_output"
grep -q 'elf_exec_test ok' <<<"$netbsd_elf_exec_output"
grep -q 'exit=0' <<<"$netbsd_elf_exec_output"
printf 'real LLVM LNP64 run-elf NetBSD ELF exec parent passed: %s\n' \
  target/llvm-lnp64-build/lnp64-netbsd-elf-exec-test-linked.elf
run_elf_report "real LLVM LNP64 run-elf NetBSD fork/wait child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-fork-wait-test-linked.elf \
  'fork_wait_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD thread child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-thread-test-linked.elf \
  'thread_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD poll child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-poll-test-linked.elf \
  'poll_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD signal gate child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-signal-gate-test-linked.elf \
  'signal_gate_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD signal fault child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-signal-fault-test-linked.elf \
  'signal_fault_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD timer child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-timer-test-linked.elf \
  'timer_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD mmap child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-mmap-test-linked.elf \
  'mmap_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD fd passing child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-fd-passing-test-linked.elf \
  'fd_passing_test ok'
netbsd_namespace_fixture_root="target/llvm-lnp64-build/netbsd-namespace-fixture-root"
rm -rf "$netbsd_namespace_fixture_root"
mkdir -p "$netbsd_namespace_fixture_root/etc" "$netbsd_namespace_fixture_root/tmp"
printf 'welcome\n' >"$netbsd_namespace_fixture_root/etc/motd"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-netbsd-namespace-test-linked.elf \
  >/dev/null
netbsd_namespace_output="$("$lnp64_bin" run-elf --namespace-root "$netbsd_namespace_fixture_root" \
  target/llvm-lnp64-build/lnp64-netbsd-namespace-test-linked.elf)"
grep -q 'namespace_test ok' <<<"$netbsd_namespace_output"
grep -q 'exit=0' <<<"$netbsd_namespace_output"
printf 'real LLVM LNP64 run-elf NetBSD namespace child passed: %s\n' \
  target/llvm-lnp64-build/lnp64-netbsd-namespace-test-linked.elf
netbsd_fixture_root="target/llvm-lnp64-build/netbsd-fixture-root"
rm -rf "$netbsd_fixture_root"
mkdir -p "$netbsd_fixture_root/etc" "$netbsd_fixture_root/tmp"
netbsd_fs_image="$netbsd_fixture_root/etc/netbsd_personality.fs"
truncate -s 512 "$netbsd_fs_image"
put_netbsd_fs_image() {
  local offset="$1"
  local bytes="$2"
  printf '%b' "$bytes" | dd of="$netbsd_fs_image" bs=1 seek="$offset" conv=notrunc status=none
}
put_netbsd_fs_image 0 'LNPFS2\n0'
put_netbsd_fs_image 64 '1d11/\0'
put_netbsd_fs_image 100 'x'
put_netbsd_fs_image 128 '1d11/etc\0'
put_netbsd_fs_image 164 'x'
put_netbsd_fs_image 192 '1f11/etc/motd\0'
put_netbsd_fs_image 228 'r'
put_netbsd_fs_image 232 'welcome\0'
put_netbsd_fs_image 256 '1d11/tmp\0'
put_netbsd_fs_image 292 'x'
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-netbsd-fs-service-test-linked.elf \
  >/dev/null
netbsd_fs_service_output="$("$lnp64_bin" run-elf --namespace-root "$netbsd_fixture_root" \
  target/llvm-lnp64-build/lnp64-netbsd-fs-service-test-linked.elf)"
grep -q 'fs_service_test ok' <<<"$netbsd_fs_service_output"
grep -q 'exit=0' <<<"$netbsd_fs_service_output"
printf 'real LLVM LNP64 run-elf NetBSD fs service child passed: %s\n' \
  target/llvm-lnp64-build/lnp64-netbsd-fs-service-test-linked.elf
run_elf_report "real LLVM LNP64 run-elf NetBSD classifier child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-classifier-test-linked.elf \
  'classifier_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD socket loopback child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-socket-loopback-test-linked.elf \
  'socket_loopback_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD gate trace child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-gate-trace-test-linked.elf \
  'gate_trace_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD domain nested child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-domain-nested-test-linked.elf \
  'domain_nested_test ok'
run_elf_report "real LLVM LNP64 run-elf NetBSD domain budget child passed" \
  target/llvm-lnp64-build/lnp64-netbsd-domain-budget-test-linked.elf \
  'domain_budget_test ok'
LNP64_LLVM_PACKAGE_FILTER=netbsd bash scripts/run_real_llvm_package_gate.sh
printf '%s\n' "real LLVM LNP64 run-elf NetBSD package/system gate passed"
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
printf 'one two\nthree\n' >"$sbase_fixture_root/input/wc.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-wc-linked.elf \
  >/dev/null
sbase_wc_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-wc-linked.elf wc input/wc.txt)"
grep -q '^2 3 14 input/wc.txt$' <<<"$sbase_wc_output"
grep -q 'exit=0' <<<"$sbase_wc_output"
printf 'real LLVM LNP64 run-elf sbase wc execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-wc-linked.elf
printf 'alpha\nbeta\ngamma\n' >"$sbase_fixture_root/input/head.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-head-linked.elf \
  >/dev/null
sbase_head_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-head-linked.elf head -n 2 input/head.txt)"
grep -q '^alpha$' <<<"$sbase_head_output"
grep -q '^beta$' <<<"$sbase_head_output"
if grep -q '^gamma$' <<<"$sbase_head_output"; then
  printf 'sbase head printed too many lines\n' >&2
  exit 1
fi
grep -q 'exit=0' <<<"$sbase_head_output"
printf 'real LLVM LNP64 run-elf sbase head execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-head-linked.elf
printf 'same via clang\n' >"$sbase_fixture_root/input/cmp-a.txt"
cp "$sbase_fixture_root/input/cmp-a.txt" "$sbase_fixture_root/input/cmp-b.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-cmp-linked.elf \
  >/dev/null
sbase_cmp_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-cmp-linked.elf cmp input/cmp-a.txt input/cmp-b.txt)"
grep -q 'exit=0' <<<"$sbase_cmp_output"
printf 'real LLVM LNP64 run-elf sbase cmp execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-cmp-linked.elf
printf 'cksum via clang\n' >"$sbase_fixture_root/input/cksum.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-cksum-linked.elf \
  >/dev/null
sbase_cksum_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-cksum-linked.elf cksum input/cksum.txt)"
grep -q '^622224091 16 input/cksum.txt$' <<<"$sbase_cksum_output"
grep -q 'exit=0' <<<"$sbase_cksum_output"
printf 'real LLVM LNP64 run-elf sbase cksum execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-cksum-linked.elf
printf 'alpha\nalpha\nbeta\nbeta\nalpha\n' >"$sbase_fixture_root/input/uniq.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-uniq-linked.elf \
  >/dev/null
sbase_uniq_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-uniq-linked.elf uniq input/uniq.txt)"
test "$(grep -c '^alpha$' <<<"$sbase_uniq_output")" -eq 2
test "$(grep -c '^beta$' <<<"$sbase_uniq_output")" -eq 1
grep -q 'exit=0' <<<"$sbase_uniq_output"
printf 'real LLVM LNP64 run-elf sbase uniq execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-uniq-linked.elf
printf 'one\ntwo\nthree\nfour\n' >"$sbase_fixture_root/input/tail.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-tail-linked.elf \
  >/dev/null
sbase_tail_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-tail-linked.elf tail -n 2 input/tail.txt)"
grep -q '^three$' <<<"$sbase_tail_output"
grep -q '^four$' <<<"$sbase_tail_output"
if grep -q '^two$' <<<"$sbase_tail_output"; then
  printf 'sbase tail printed too many lines\n' >&2
  exit 1
fi
grep -q 'exit=0' <<<"$sbase_tail_output"
printf 'real LLVM LNP64 run-elf sbase tail execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-tail-linked.elf
rm -f "$sbase_fixture_root/tee-copy.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-tee-linked.elf \
  >/dev/null
printf 'tee via clang\n' | "$lnp64_bin" run-elf \
  --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-tee-linked.elf tee tee-copy.txt \
  >"$sbase_fixture_root/tee-stdout.txt"
grep -q '^tee via clang$' "$sbase_fixture_root/tee-stdout.txt"
grep -q 'exit=0' "$sbase_fixture_root/tee-stdout.txt"
grep -q '^tee via clang$' "$sbase_fixture_root/tee-copy.txt"
printf 'real LLVM LNP64 run-elf sbase tee execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-tee-linked.elf
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-ls-linked.elf \
  >/dev/null
sbase_ls_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-ls-linked.elf ls input)"
grep -q '^cat.txt$' <<<"$sbase_ls_output"
grep -q 'exit=0' <<<"$sbase_ls_output"
printf 'real LLVM LNP64 run-elf sbase ls execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-ls-linked.elf
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-find-linked.elf \
  >/dev/null
sbase_find_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-find-linked.elf find input -name cat.txt -print)"
grep -q '^input/cat.txt$' <<<"$sbase_find_output"
grep -q 'exit=0' <<<"$sbase_find_output"
printf 'real LLVM LNP64 run-elf sbase find execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-find-linked.elf
rm -rf "$sbase_fixture_root/made"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-mkdir-linked.elf \
  >/dev/null
sbase_mkdir_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-mkdir-linked.elf mkdir made)"
grep -q 'exit=0' <<<"$sbase_mkdir_output"
test -d "$sbase_fixture_root/made"
printf 'real LLVM LNP64 run-elf sbase mkdir execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-mkdir-linked.elf
rm -f "$sbase_fixture_root/linked.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-ln-linked.elf \
  >/dev/null
sbase_ln_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-ln-linked.elf ln input/cat.txt linked.txt)"
grep -q 'exit=0' <<<"$sbase_ln_output"
cmp -s "$sbase_fixture_root/input/cat.txt" "$sbase_fixture_root/linked.txt"
printf 'real LLVM LNP64 run-elf sbase ln execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-ln-linked.elf
printf 'mode via clang\n' >"$sbase_fixture_root/chmod.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-chmod-linked.elf \
  >/dev/null
sbase_chmod_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-chmod-linked.elf chmod 700 chmod.txt)"
grep -q 'exit=0' <<<"$sbase_chmod_output"
test "$(stat -c '%a' "$sbase_fixture_root/chmod.txt")" = 700
printf 'real LLVM LNP64 run-elf sbase chmod execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-chmod-linked.elf
printf 'chown via clang\n' >"$sbase_fixture_root/chown.txt"
chmod 644 "$sbase_fixture_root/chown.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-chown-linked.elf \
  >/dev/null
sbase_chown_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-chown-linked.elf chown :"$(id -g)" chown.txt)"
grep -q 'exit=0' <<<"$sbase_chown_output"
test "$(stat -c '%g' "$sbase_fixture_root/chown.txt")" = "$(id -g)"
printf 'real LLVM LNP64 run-elf sbase chown execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-chown-linked.elf
rm -f "$sbase_fixture_root/touched.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-touch-linked.elf \
  >/dev/null
sbase_touch_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-touch-linked.elf touch touched.txt)"
grep -q 'exit=0' <<<"$sbase_touch_output"
test -f "$sbase_fixture_root/touched.txt"
printf 'real LLVM LNP64 run-elf sbase touch execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-touch-linked.elf
printf 'move via clang\n' >"$sbase_fixture_root/move-source.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-mv-linked.elf \
  >/dev/null
sbase_mv_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-mv-linked.elf mv move-source.txt moved.txt)"
grep -q 'exit=0' <<<"$sbase_mv_output"
test ! -e "$sbase_fixture_root/move-source.txt"
grep -q 'move via clang' "$sbase_fixture_root/moved.txt"
printf 'real LLVM LNP64 run-elf sbase mv execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-mv-linked.elf
printf 'remove via clang\n' >"$sbase_fixture_root/remove.txt"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-sbase-rm-linked.elf \
  >/dev/null
sbase_rm_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
  target/llvm-lnp64-build/lnp64-sbase-rm-linked.elf rm remove.txt)"
grep -q 'exit=0' <<<"$sbase_rm_output"
test ! -e "$sbase_fixture_root/remove.txt"
printf 'real LLVM LNP64 run-elf sbase rm execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-sbase-rm-linked.elf
userland_fixture_root="target/llvm-lnp64-build/userland-fixture-root"
mkdir -p "$userland_fixture_root/dev" "$userland_fixture_root/etc"
printf 'welcome from clang ucat\n' >"$userland_fixture_root/etc/motd"
printf 'console\nnull\nrandom\n' >"$userland_fixture_root/dev/devices"
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-userland-ucat-linked.elf \
  >/dev/null
userland_ucat_output="$("$lnp64_bin" run-elf --namespace-root "$userland_fixture_root" \
  target/llvm-lnp64-build/lnp64-userland-ucat-linked.elf ucat etc/motd)"
grep -q '^welcome from clang ucat$' <<<"$userland_ucat_output"
grep -q 'exit=0' <<<"$userland_ucat_output"
printf 'real LLVM LNP64 run-elf userland ucat execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-userland-ucat-linked.elf
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-userland-init-linked.elf \
  >/dev/null
userland_init_output="$("$lnp64_bin" run-elf --namespace-root "$userland_fixture_root" \
  target/llvm-lnp64-build/lnp64-userland-init-linked.elf init /)"
grep -q '^lnp64 clang init: boot$' <<<"$userland_init_output"
grep -q '^lnp64 clang init: root /$' <<<"$userland_init_output"
grep -q '^welcome from clang ucat$' <<<"$userland_init_output"
grep -q 'exit=0' <<<"$userland_init_output"
printf 'real LLVM LNP64 run-elf userland init execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-userland-init-linked.elf
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-userland-lnpsh-linked.elf \
  >/dev/null
userland_lnpsh_output="$("$lnp64_bin" run-elf --namespace-root "$userland_fixture_root" \
  target/llvm-lnp64-build/lnp64-userland-lnpsh-linked.elf lnpsh)"
grep -q '^lnpsh clang: scripted console$' <<<"$userland_lnpsh_output"
grep -q '^welcome from clang ucat$' <<<"$userland_lnpsh_output"
grep -q '^console$' <<<"$userland_lnpsh_output"
grep -q '^random$' <<<"$userland_lnpsh_output"
grep -q 'exit=0' <<<"$userland_lnpsh_output"
printf 'real LLVM LNP64 run-elf userland lnpsh execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-userland-lnpsh-linked.elf
"$lnp64_bin" elf-plan target/llvm-lnp64-build/lnp64-userland-spawn-task-linked.elf \
  >/dev/null
userland_spawn_output="$("$lnp64_bin" run-elf \
  target/llvm-lnp64-build/lnp64-userland-spawn-task-linked.elf spawn-task)"
grep -q '^userland spawn: parent$' <<<"$userland_spawn_output"
grep -q '^userland spawn: child$' <<<"$userland_spawn_output"
grep -q '^userland spawn: joined$' <<<"$userland_spawn_output"
grep -q 'exit=0' <<<"$userland_spawn_output"
printf 'real LLVM LNP64 run-elf userland spawn task execution passed: %s\n' \
  target/llvm-lnp64-build/lnp64-userland-spawn-task-linked.elf
run_elf_report "real LLVM LNP64 run-elf indirect call execution passed" \
  target/llvm-lnp64-build/lnp64-indirect-call-linked.elf

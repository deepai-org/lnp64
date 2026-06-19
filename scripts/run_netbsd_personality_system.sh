#!/usr/bin/env bash
set -euo pipefail

mode="llvm"
usage() {
  cat <<'USAGE'
usage: scripts/run_netbsd_personality_system.sh [--backend llvm|toy] [--legacy-toy]

The default llvm backend runs the Clang/lld/run-elf NetBSD personality probes
through the real LLVM package gate. --legacy-toy preserves the old integrated
host-directory script that compiles C with lnp64 cc --toy-bootstrap.
USAGE
}

while (($#)); do
  case "$1" in
    --backend)
      mode="${2:-}"
      if [[ -z "$mode" ]]; then
        printf '%s\n' "missing value for --backend" >&2
        usage >&2
        exit 2
      fi
      shift 2
      ;;
    --legacy-toy)
      mode="toy"
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

case "$mode" in
  llvm)
    printf '%s\n' "== real LLVM LNP64 package gate: netbsd =="
    LNP64_LLVM_PACKAGE_FILTER=netbsd bash scripts/run_real_llvm_package_gate.sh
    printf '%s\n' "netbsd personality clang gate ok"
    exit 0
    ;;
  toy)
    ;;
  *)
    printf 'unknown backend: %s\n' "$mode" >&2
    usage >&2
    exit 2
    ;;
esac

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi
root="${TMPDIR:-/tmp}/lnp64-netbsd-personality-root"
out="${TMPDIR:-/tmp}/lnp64-netbsd-personality.out"
expected="${TMPDIR:-/tmp}/lnp64-netbsd-personality.expected"
trace="${TMPDIR:-/tmp}/lnp64-netbsd-personality.trace"

programs=(
  netbsd_sh
  loader_target
  thread_test
  namespace_test
  loader_test
  poll_test
  fs_service_test
  mmap_test
  fd_passing_test
  gate_trace_test
  timer_test
  classifier_test
  socket_loopback_test
  signal_gate_test
  signal_fault_test
  domain_nested_test
  domain_budget_test
)

rm -rf "$root"
mkdir -p "$root/bin" "$root/dev" "$root/etc" "$root/sbin" "$root/tmp"

cat > "$root/etc/motd" <<'MOTD'
welcome to lnp64-netbsd-personality
MOTD

cat > "$root/etc/loader_target.execplan" <<'PLAN'
LNP64EXEC1
/bin/loader_target.s
PLAN
cat > "$root/etc/loader_bad_magic.execplan" <<'PLAN'
BADPLAN1
/bin/loader_target.s
PLAN
cat > "$root/etc/loader_missing_path.execplan" <<'PLAN'
LNP64EXEC1
PLAN
cat > "$root/etc/loader_empty_path.execplan" <<'PLAN'
LNP64EXEC1

PLAN
cat > "$root/etc/loader_relative_path.execplan" <<'PLAN'
LNP64EXEC1
bin/loader_target.s
PLAN

fs_image="$root/etc/netbsd_personality.fs"
truncate -s 512 "$fs_image"
put_image() {
  local offset="$1"
  local bytes="$2"
  printf '%b' "$bytes" | dd of="$fs_image" bs=1 seek="$offset" conv=notrunc status=none
}
put_image 0 'LNPFS2\n0'
put_image 64 '1d11/\0'
put_image 100 'x'
put_image 128 '1d11/etc\0'
put_image 164 'x'
put_image 192 '1f11/etc/motd\0'
put_image 228 'r'
put_image 232 'welcome\0'
put_image 256 '1d11/tmp\0'
put_image 292 'x'

: > "$trace"
"${lnp64[@]}" cc --toy-bootstrap userland/netbsd_init.c -o "$root/sbin/init.s"
cat "$root/sbin/init.s" >> "$trace"
for program in "${programs[@]}"; do
  "${lnp64[@]}" cc --toy-bootstrap "userland/${program}.c" -o "$root/bin/${program}.s"
  cat "$root/bin/${program}.s" >> "$trace"
done
cat > "$root/bin/bad_exec.s" <<'ASM'
.text
  BAD_OPCODE r1
ASM

"${lnp64[@]}" run --namespace-root "$root" "$root/sbin/init.s" -- init / > "$out"

cat > "$expected" <<EXPECTED
lnp64-netbsd-personality: supervisor boot
lnp64-netbsd-personality: root /
/init
/bin/sh -c 'netbsd personality system script'
$ echo hello > /tmp/a
$ cat /tmp/a | wc
1 1 6
$ mkdir /tmp/d
$ ls /tmp
a
d
$ ./thread_test
thread_test ok
$ ./namespace_test
namespace_test ok
$ ./loader_test
loader_target ok
loader_test ok
$ ./poll_test
poll_test ok
$ ./fs_service_test
fs_service_test ok
$ ./mmap_test
mmap_test ok
$ ./fd_passing_test
fd_passing_test ok
$ ./gate_trace_test
gate_trace_test ok
$ ./timer_test
timer_test ok
$ ./classifier_test
classifier_test ok
$ ./socket_loopback_test
socket_loopback_test ok
$ ./signal_gate_test
signal_gate_test ok
$ ./signal_fault_test
signal_fault_test ok
$ ./domain_nested_test
domain_nested_test ok
$ ./domain_budget_test
domain_budget_test ok
netbsd personality system ok
EXPECTED

diff -u "$expected" "$out"

required_native=(
  OPEN_AT
  PULL_DYN
  PUSH_DYN
  CHDIR_PATH
  GETCWD_PATH
  MMAP
  MPROTECT
  MUNMAP
  AWAIT_DYN
  OBJECT_CTL
  GATE_CALL
  GATE_RETURN
  DOMAIN_CTL
  CAP_DUP
  CAP_SEND
  CAP_RECV
  FORK
  EXEC
  SPAWN
  POLL_FD_DYN
  PWRITE_FD_DYN
  FD_SEEK
  SLEEP
  ALARM
  SIGACTION
  KILL
)

for token in "${required_native[@]}"; do
  grep -q "$token" "$trace"
done

legacy_aliases=(
  MSG_RECV
  PIPE
  OPEN_FD
  OPEN_FD_DYN
  READ_FD_DYN
  WRITE_FD_DYN
  EVENT_CTL
  TIMER_CTL
  CALL_CAP
  RET_CAP
)

for token in "${legacy_aliases[@]}"; do
  if grep -Eq "\\b${token}\\b" "$trace"; then
    printf 'legacy primitive alias in generated trace: %s\n' "$token" >&2
    exit 1
  fi
done

cargo test --quiet netbsd_system_gate_canonical_native_primitives_cover_runner_requirements

for forbidden in IRQ MMIO DMA_CTL PAGE_TABLE SCHED_CTL RAW_SYSCALL; do
  if grep -q "$forbidden" "$trace"; then
    printf 'forbidden primitive in trace: %s\n' "$forbidden" >&2
    exit 1
  fi
done

grep -q "SIGRET" "$trace"
if grep -q "RAW_SIGNAL" "$trace"; then
  printf '%s\n' "raw signal primitive appeared in trace" >&2
  exit 1
fi

"${lnp64[@]}" run demos/stale_fd_token.s > "${TMPDIR:-/tmp}/lnp64-netbsd-stale.out"
grep -q "stale fd token ok" "${TMPDIR:-/tmp}/lnp64-netbsd-stale.out"

cat "$out"
printf '%s\n' "netbsd personality system gate ok"

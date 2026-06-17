#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --release --quiet --)
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
"${lnp64[@]}" cc userland/netbsd_init.c -o "$root/sbin/init.s"
cat "$root/sbin/init.s" >> "$trace"
for program in "${programs[@]}"; do
  "${lnp64[@]}" cc "userland/${program}.c" -o "$root/bin/${program}.s"
  cat "$root/bin/${program}.s" >> "$trace"
done

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
  OPEN_FD
  READ_FD_DYN
  WRITE_FD_DYN
  CHDIR_PATH
  GETCWD_PATH
  MMAP
  MPROTECT
  MUNMAP
  AWAIT_DYN
  OBJECT_CTL
  CALL_CAP
  RET_CAP
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

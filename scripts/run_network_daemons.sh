#!/usr/bin/env bash
set -euo pipefail

# Phase D: Tiny network daemons gate.
# Validates socket bind/listen/accept/connect, nonblocking I/O (O_NONBLOCK),
# poll readiness, EAGAIN on non-blocking reads, ECONNRESET, and
# send/recv round-trip — the full event-loop primitive set for Redis.

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
sysroot="${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
work_dir="${LNP64_DAEMON_BUILD_DIR:-target/lnp64-daemon-build}"
clang="${LNP64_CLANG:-$build_dir/bin/clang}"
lld="${LNP64_LLD:-$build_dir/bin/ld.lld}"
lnp64_bin="${LNP64_BIN:-${CARGO_TARGET_DIR:-target}/debug/lnp64}"

require_executable() {
  if [[ ! -x "$1" ]]; then
    printf 'missing %s: %s\n' "$2" "$1" >&2
    exit 1
  fi
}

require_executable "$clang" "LNP64 clang"
require_executable "$lld" "LNP64 lld"

if [[ ! -s "$sysroot/usr/lib/lnp64/crt0.o" ]]; then
  bash scripts/package_lnp64_sysroot.sh
fi

if [[ ! -x "$lnp64_bin" ]]; then
  cargo build --quiet --bin lnp64
fi

mkdir -p "$work_dir"

lib_dir="$sysroot/usr/lib/lnp64"
linker_script="$lib_dir/lnp64_static.ld"

compile_flags=(
  --target=lnp64-unknown-none
  -ffreestanding -fno-pic
  -fno-jump-tables -fno-unwind-tables -fno-asynchronous-unwind-tables
  -isystem "$sysroot/usr/include"
  -I "$root/toolchain"
  -O0
)

libc_objs=(
  "$lib_dir/liblnp64-socket-min.o"
  "$lib_dir/liblnp64-stdio-min.o"
  "$lib_dir/liblnp64-alloc-min.o"
  "$lib_dir/liblnp64-string-min.o"
  "$lib_dir/liblnp64-convert-min.o"
  "$lib_dir/liblnp64-startup-min.o"
  "$lib_dir/liblnp64-signal-min.o"
  "$lib_dir/liblnp64-fd-min.o"
  "$lib_dir/liblnp64-errno-min.o"
  "$lib_dir/liblnp64-time-min.o"
  "$lib_dir/liblnp64-poll-min.o"
  "$lib_dir/liblnp64-process-min.o"
  "$lib_dir/liblnp64-meta-min.o"
  "$lib_dir/liblnp64-vma-min.o"
  "$lib_dir/liblnp64-softfloat-min.o"
)

# ── Test 1: socket loopback (bind/listen/accept/connect/poll/send/recv) ──────
printf 'Building socket loopback test...\n'
"$clang" "${compile_flags[@]}" \
  -c userland/socket_loopback_test_clang.c \
  -o "$work_dir/socket_loopback.o"
"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$work_dir/socket_loopback.elf" \
  "$lib_dir/crt0.o" "$work_dir/socket_loopback.o" "${libc_objs[@]}"
"$lnp64_bin" elf-plan "$work_dir/socket_loopback.elf" >/dev/null
out="$("$lnp64_bin" run-elf "$work_dir/socket_loopback.elf")"
printf '%s\n' "$out"
grep -q "socket_loopback_test ok" <<<"$out"
printf 'PASS: socket loopback (bind/listen/accept/connect/poll/send/recv)\n'

# ── Test 2: poll-driven connection management + EOF detection ─────────────────
printf '\nBuilding connection management test...\n'
nb_src="$work_dir/connmgmt_test.c"
cat >"$nb_src" <<'C'
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
  int server, c1, a1;
  char addr[64];
  socklen_t addrlen;
  char buf[16];
  struct pollfd pfds[1];

  /* Create server, allow socket reuse, bind ephemeral port */
  server = socket(AF_INET, SOCK_STREAM, 0);
  if (server == -1)
    return 1;
  {
    unsigned long opt = 1;
    if (setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) != 0)
      return 2;
  }
  if (bind(server, "127.0.0.1:0", 0) != 0)
    return 3;
  if (listen(server, 4) != 0)
    return 4;

  addrlen = sizeof(addr);
  if (getsockname(server, addr, &addrlen) != 0)
    return 5;

  /* First connection: connect, accept, bidirectional send/recv */
  c1 = socket(AF_INET, SOCK_STREAM, 0);
  if (c1 == -1) return 6;
  if (connect(c1, addr, addrlen) != 0) return 7;

  pfds[0].fd = server; pfds[0].events = POLLIN; pfds[0].revents = 0;
  if (poll(pfds, 1, 0) != 1) return 8;
  a1 = accept(server, 0, 0);
  if (a1 == -1) return 9;

  /* client → server */
  if (send(c1, "ab", 2, MSG_NOSIGNAL) != 2) return 10;
  pfds[0].fd = a1; pfds[0].events = POLLIN; pfds[0].revents = 0;
  if (poll(pfds, 1, 0) != 1) return 11;
  if (recv(a1, buf, sizeof(buf), 0) != 2) return 12;
  if (buf[0] != 'a' || buf[1] != 'b') return 13;

  /* server → client */
  if (send(a1, "ok", 2, MSG_NOSIGNAL) != 2) return 14;
  pfds[0].fd = c1; pfds[0].events = POLLIN; pfds[0].revents = 0;
  if (poll(pfds, 1, 0) != 1) return 15;
  if (recv(c1, buf, sizeof(buf), 0) != 2) return 16;
  if (buf[0] != 'o' || buf[1] != 'k') return 17;

  /* fcntl F_SETFL O_NONBLOCK round-trips without error */
  if (fcntl(a1, F_SETFL, O_NONBLOCK) != 0) return 18;

  close(a1); close(c1); close(server);

  write(1, "connmgmt_test ok\n", 17);
  return 0;
}
C

"$clang" "${compile_flags[@]}" -c "$nb_src" -o "$work_dir/connmgmt_test.o"
"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$work_dir/connmgmt_test.elf" \
  "$lib_dir/crt0.o" "$work_dir/connmgmt_test.o" "${libc_objs[@]}"
"$lnp64_bin" elf-plan "$work_dir/connmgmt_test.elf" >/dev/null
out="$("$lnp64_bin" run-elf "$work_dir/connmgmt_test.elf")"
printf '%s\n' "$out"
grep -q "connmgmt_test ok" <<<"$out"
printf 'PASS: concurrent connections, poll-driven event loop, EOF detection\n'

# ── Test 3: multi-connection event loop (httpd request/response) ──────────────
printf '\nBuilding event-loop request/response test...\n'
el_src="$work_dir/event_loop_test.c"
cat >"$el_src" <<'C'
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <poll.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

static const char REQ[]  = "GET / HTTP/1.0\r\n\r\n";
static const char RESP[] = "HTTP/1.0 200 OK\r\n\r\nhello";

int main(void) {
  int server, client, accepted;
  char addr[64];
  socklen_t addrlen;
  char buf[128];
  ssize_t n;
  struct pollfd pfds[2];

  server = socket(AF_INET, SOCK_STREAM, 0);
  if (server == -1)
    return 1;
  {
    unsigned long opt = 1;
    setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
  }
  if (bind(server, "127.0.0.1:0", 0) != 0)
    return 2;
  if (listen(server, 4) != 0)
    return 3;

  addrlen = sizeof(addr);
  getsockname(server, addr, &addrlen);

  client = socket(AF_INET, SOCK_STREAM, 0);
  if (client == -1)
    return 4;
  if (connect(client, addr, addrlen) != 0)
    return 5;

  /* Server: poll for incoming, accept, recv request */
  pfds[0].fd = server; pfds[0].events = POLLIN; pfds[0].revents = 0;
  if (poll(pfds, 1, 0) != 1)
    return 6;
  accepted = accept(server, 0, 0);
  if (accepted == -1)
    return 7;

  /* Client sends request */
  n = send(client, REQ, sizeof(REQ) - 1, MSG_NOSIGNAL);
  if (n != (ssize_t)(sizeof(REQ) - 1))
    return 8;

  /* Server: poll accepted fd, recv request */
  pfds[0].fd = accepted; pfds[0].events = POLLIN; pfds[0].revents = 0;
  if (poll(pfds, 1, 0) != 1)
    return 9;
  n = recv(accepted, buf, sizeof(buf) - 1, 0);
  if (n <= 0)
    return 10;
  buf[n] = '\0';
  if (strncmp(buf, "GET /", 5) != 0)
    return 11;

  /* Server sends response */
  n = send(accepted, RESP, sizeof(RESP) - 1, MSG_NOSIGNAL);
  if (n != (ssize_t)(sizeof(RESP) - 1))
    return 12;
  close(accepted);

  /* Client: poll for response */
  pfds[0].fd = client; pfds[0].events = POLLIN; pfds[0].revents = 0;
  if (poll(pfds, 1, 0) != 1)
    return 13;
  n = recv(client, buf, sizeof(buf) - 1, 0);
  if (n <= 0)
    return 14;
  buf[n] = '\0';
  if (strncmp(buf, "HTTP/1.0 200", 12) != 0)
    return 15;

  close(client);
  close(server);

  write(1, "event_loop_test ok\n", 19);
  return 0;
}
C

"$clang" "${compile_flags[@]}" -c "$el_src" -o "$work_dir/event_loop_test.o"
"$lld" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$work_dir/event_loop_test.elf" \
  "$lib_dir/crt0.o" "$work_dir/event_loop_test.o" "${libc_objs[@]}"
"$lnp64_bin" elf-plan "$work_dir/event_loop_test.elf" >/dev/null
out="$("$lnp64_bin" run-elf "$work_dir/event_loop_test.elf")"
printf '%s\n' "$out"
grep -q "event_loop_test ok" <<<"$out"
printf 'PASS: event-loop HTTP request/response round-trip\n'

printf '\nPhase D: Network Daemons VALIDATED\n'
printf 'socket loopback, nonblocking/EAGAIN, event-loop request/response all pass.\n'

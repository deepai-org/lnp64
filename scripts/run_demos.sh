#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi

non_network=(
  demos/allocator.c
  demos/cat.c
  demos/factorial.c
  demos/fibonacci.c
  demos/hello.c
  demos/json_parser.c
  demos/netbsd_personality_smoke.c
  demos/parallel_hash.c
  demos/pcr.c
  demos/ping_pong.c
  demos/producer_consumer.c
  demos/rot13.c
  demos/sqlite_lite.c
)

for src in "${non_network[@]}"; do
  asm="/tmp/$(basename "$src" .c).s"
  "${lnp64[@]}" cc --toy-bootstrap "$src" -o "$asm"
  echo "== $src =="
  "${lnp64[@]}" run "$asm"
done

echo "== demos/netcat.c =="
"${lnp64[@]}" cc --toy-bootstrap demos/netcat.c -o /tmp/netcat.s
rm -f /tmp/netcat.out
"${lnp64[@]}" run /tmp/netcat.s > /tmp/netcat.out &
netcat_pid=$!
for _ in $(seq 1 50); do
  grep -q "netcat ready" /tmp/netcat.out 2>/dev/null && break
  sleep 0.1
done
exec 9<>/dev/tcp/127.0.0.1/41065
printf 'netcat ok\n' >&9
IFS= read -r netcat_reply <&9
exec 9>&-
wait "$netcat_pid"
cat /tmp/netcat.out
test "$netcat_reply" = "netcat ok"

echo "== demos/httpd.c =="
"${lnp64[@]}" cc --toy-bootstrap demos/httpd.c -o /tmp/httpd.s
rm -f /tmp/httpd.out /tmp/httpd.response
"${lnp64[@]}" run /tmp/httpd.s > /tmp/httpd.out &
httpd_pid=$!
for _ in $(seq 1 50); do
  grep -q "httpd ready" /tmp/httpd.out 2>/dev/null && break
  sleep 0.1
done
exec 8<>/dev/tcp/127.0.0.1/41066
printf 'GET / HTTP/1.1\r\nHost: localhost\r\n\r\n' >&8
dd bs=1 count=55 <&8 > /tmp/httpd.response 2>/dev/null || true
exec 8>&-
wait "$httpd_pid"
cat /tmp/httpd.out
cat /tmp/httpd.response
grep -q "hello from http" /tmp/httpd.response

for src in demos/*.s; do
  echo "== $src =="
  "${lnp64[@]}" run "$src"
done

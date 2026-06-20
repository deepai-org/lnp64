#!/usr/bin/env bash
# Phase E gate: Redis 7.0.15 smoke test on LNP64.
# Runs redis-server.elf in background, sends PING/SET/GET/DEL via redis-cli,
# verifies expected responses, then shuts the server down.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lnp64_bin="${LNP64_BIN:-${root}/target/debug/lnp64}"
redis_elf="${root}/target/redis-lnp64-build/redis-server.elf"
port=16379
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

if [[ ! -f "$redis_elf" ]]; then
  echo "FAIL: redis-server.elf not found — run scripts/build_redis.sh first" >&2
  exit 1
fi
if [[ ! -x "$lnp64_bin" ]]; then
  echo "FAIL: lnp64 binary not found at $lnp64_bin" >&2
  exit 1
fi

echo "=== Redis LNP64 smoke test ==="

# Start redis-server in the LNP64 emulator; use minimal config
config="${tmpdir}/redis.conf"
cat >"$config" <<EOF
port $port
daemonize no
loglevel warning
logfile ""
save ""
appendonly no
protected-mode no
EOF

# Launch server in background, capture output
server_log="${tmpdir}/server.log"
"$lnp64_bin" run-elf "$redis_elf" -- --port "$port" --save "" \
  >"$server_log" 2>&1 &
server_pid=$!

# Wait for server to be ready (up to 10 s)
for i in $(seq 1 50); do
  if redis-cli -p "$port" PING 2>/dev/null | grep -q PONG; then
    break
  fi
  sleep 0.2
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "FAIL: redis-server exited early. Log:"
    cat "$server_log" | tail -20
    exit 1
  fi
done

if ! redis-cli -p "$port" PING 2>/dev/null | grep -q PONG; then
  echo "FAIL: redis-server did not respond to PING within 10s. Log:"
  cat "$server_log" | tail -20
  kill "$server_pid" 2>/dev/null || true
  exit 1
fi

echo "  PING → PONG  OK"

# SET
result=$(redis-cli -p "$port" SET testkey hello)
[[ "$result" == "OK" ]] || { echo "FAIL: SET returned '$result'"; kill "$server_pid"; exit 1; }
echo "  SET testkey hello → OK"

# GET
result=$(redis-cli -p "$port" GET testkey)
[[ "$result" == "hello" ]] || { echo "FAIL: GET returned '$result'"; kill "$server_pid"; exit 1; }
echo "  GET testkey → hello  OK"

# DEL
result=$(redis-cli -p "$port" DEL testkey)
[[ "$result" == "1" ]] || { echo "FAIL: DEL returned '$result'"; kill "$server_pid"; exit 1; }
echo "  DEL testkey → 1  OK"

# Verify key is gone
result=$(redis-cli -p "$port" GET testkey)
[[ -z "$result" ]] || { echo "FAIL: GET after DEL returned '$result'"; kill "$server_pid"; exit 1; }
echo "  GET testkey (after DEL) → (nil)  OK"

# Shutdown
redis-cli -p "$port" SHUTDOWN NOSAVE 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true

echo "=== Redis smoke test PASSED ==="

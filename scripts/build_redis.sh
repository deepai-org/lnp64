#!/bin/bash
set -euo pipefail

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
SYSROOT="/work/${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}"
CLANG="/work/$build_dir/bin/clang"
LLD="/work/$build_dir/bin/ld.lld"
OUT_DIR=/work/target/redis-lnp64-build

CFLAGS="--target=lnp64-unknown-none -ffreestanding -fno-pic -O0 -fno-jump-tables
  -fno-unwind-tables -fno-asynchronous-unwind-tables -std=c11 -Wno-error
  -I/work/third_party/redis/src
  -I/work/third_party/redis/deps/lua/src
  -I/work/third_party/redis/deps/hdr_histogram
  -isystem $SYSROOT/usr/include"

LUA_CFLAGS="--target=lnp64-unknown-none -ffreestanding -fno-pic -O0 -fno-jump-tables
  -fno-unwind-tables -fno-asynchronous-unwind-tables -std=c99 -Wno-error
  -I/work/third_party/redis/deps/lua/src
  -isystem $SYSROOT/usr/include
  -DLUA_USE_POSIX"

HDR_CFLAGS="--target=lnp64-unknown-none -ffreestanding -fno-pic -O0 -fno-jump-tables
  -fno-unwind-tables -fno-asynchronous-unwind-tables -std=c11 -Wno-error
  -I/work/third_party/redis/deps/hdr_histogram
  -I/work/third_party/redis/src
  -isystem $SYSROOT/usr/include"

mkdir -p "$OUT_DIR/lua" "$OUT_DIR/hdr" "$OUT_DIR/redis"

echo "=== Building Lua ==="
LUA_SRC=/work/third_party/redis/deps/lua/src
LUA_SKIP="lua.c luac.c"
lua_ok=0
for src in "$LUA_SRC"/*.c; do
  base=$(basename "$src")
  case " $LUA_SKIP " in *" $base "*) continue;; esac
  if $CLANG $LUA_CFLAGS -c "$src" -o "$OUT_DIR/lua/${base%.c}.o" 2>/dev/null; then
    lua_ok=$((lua_ok+1))
  else
    echo "  WARN: Lua $base failed (continuing)"
  fi
done
llvm-ar rcs "$OUT_DIR/liblua.a" "$OUT_DIR/lua"/*.o
echo "  Lua built: $lua_ok objects"

echo "=== Building hdr_histogram ==="
HDR_SRC=/work/third_party/redis/deps/hdr_histogram
hdr_ok=0
for src in "$HDR_SRC"/*.c; do
  base=$(basename "$src")
  if $CLANG $HDR_CFLAGS -c "$src" -o "$OUT_DIR/hdr/${base%.c}.o" 2>/tmp/hdr_build_err.txt; then
    hdr_ok=$((hdr_ok+1))
  else
    echo "  WARN: hdr $base failed: $(grep -m1 'error:' /tmp/hdr_build_err.txt | cut -c1-100)"
  fi
done
if [ $hdr_ok -gt 0 ]; then
  llvm-ar rcs "$OUT_DIR/libhdr.a" "$OUT_DIR/hdr"/*.o
  echo "  hdr_histogram built: $hdr_ok objects"
fi

echo "=== Building Redis server objects ==="
REDIS_SRC=/work/third_party/redis/src
# Server objects from Makefile (skip tls which needs openssl, sentinel which needs hiredis)
SERVER_SRCS="adlist.c quicklist.c ae.c anet.c dict.c server.c sds.c zmalloc.c
  lzf_c.c lzf_d.c pqsort.c zipmap.c sha1.c ziplist.c release.c networking.c
  util.c object.c db.c replication.c rdb.c t_string.c t_list.c t_set.c t_zset.c
  t_hash.c config.c aof.c pubsub.c multi.c debug.c sort.c intset.c syncio.c
  cluster.c crc16.c endianconv.c slowlog.c eval.c bio.c rio.c rand.c memtest.c
  syscheck.c crcspeed.c crc64.c bitops.c notify.c setproctitle.c blocked.c
  hyperloglog.c latency.c sparkline.c redis-check-rdb.c redis-check-aof.c geo.c
  lazyfree.c module.c evict.c expire.c geohash.c geohash_helper.c childinfo.c
  defrag.c siphash.c rax.c t_stream.c listpack.c localtime.c lolwut.c lolwut5.c
  lolwut6.c acl.c tracking.c connection.c sha256.c timeout.c setcpuaffinity.c
  monotonic.c mt19937-64.c resp_parser.c call_reply.c script_lua.c script.c
  functions.c function_lua.c commands.c"

ok=0; fail=0
for base in $SERVER_SRCS; do
  src="$REDIS_SRC/$base"
  out="$OUT_DIR/redis/${base%.c}.o"
  if $CLANG $CFLAGS -c "$src" -o "$out" 2>/tmp/redis_build_err.txt; then
    ok=$((ok+1))
  else
    fail=$((fail+1))
    echo "  FAIL $base: $(grep -m1 'error:' /tmp/redis_build_err.txt | cut -c1-80)"
  fi
done
echo "  Redis objects: ok=$ok fail=$fail"

if [ $fail -gt 0 ]; then
  echo "Build FAILED: $fail compilation errors"
  exit 1
fi

echo "=== Linking redis-server.elf ==="
lib_dir="$SYSROOT/usr/lib/lnp64"
linker_script="$lib_dir/lnp64_static.ld"
crt0_obj="$lib_dir/crt0.o"
liblnp64_a="$lib_dir/liblnp64.a"

hdr_lib=""
[ -f "$OUT_DIR/libhdr.a" ] && hdr_lib="$OUT_DIR/libhdr.a"

"$LLD" -flavor gnu -static -m elf64lnp64 -T "$linker_script" \
  -o "$OUT_DIR/redis-server.elf" \
  "$crt0_obj" \
  "$OUT_DIR/redis"/*.o \
  "$OUT_DIR/liblua.a" \
  ${hdr_lib:+"$hdr_lib"} \
  --start-group "$liblnp64_a" --end-group \
  2>&1 | head -80 || { echo "Link FAILED"; exit 1; }

if [ -f "$OUT_DIR/redis-server.elf" ]; then
  echo "redis-server.elf built: $(stat -c%s "$OUT_DIR/redis-server.elf") bytes"
else
  echo "Link FAILED"
  exit 1
fi

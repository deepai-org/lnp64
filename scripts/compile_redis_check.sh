#!/bin/bash
build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
SYSROOT=/work/target/lnp64-sysroot
CLANG="/work/$build_dir/bin/clang"
CFLAGS="--target=lnp64-unknown-none -ffreestanding -fno-pic -O0 -fno-jump-tables -std=c11 -Wno-error
  -I/work/third_party/redis/src
  -I/work/third_party/redis/deps/lua/src
  -I/work/third_party/redis/deps/hdr_histogram
  -I$SYSROOT/usr/include"
REDIS_SRC=/work/third_party/redis/src

# ae_epoll/kqueue/select/evport: included inline by ae.c, not compiled standalone
# cli_common, redis-cli, redis-benchmark: need hiredis - skip for server build
# sentinel: needs hiredis
SKIP="ae_epoll.c ae_kqueue.c ae_select.c ae_evport.c cli_common.c redis-cli.c redis-benchmark.c sentinel.c"

objects=0
failures=0

mkdir -p /tmp/redis_objs
for src in "$REDIS_SRC"/*.c; do
  base=$(basename "$src")
  case " $SKIP " in *" $base "*) continue;; esac
  outobj="/tmp/redis_objs/${base%.c}.o"
  if $CLANG $CFLAGS -c "$src" -o "$outobj" 2>/tmp/redis_err.txt; then
    objects=$((objects+1))
  else
    failures=$((failures+1))
    reason=$(head -5 /tmp/redis_err.txt | grep -m1 'error:' | sed 's|.*/[^/]*:[0-9]*:[0-9]*: ||' | cut -c1-80)
    echo "  FAIL $base: $reason"
  fi
done

echo "objects: $objects, failures: $failures"

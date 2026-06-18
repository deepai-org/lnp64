#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi
root="${TMPDIR:-/tmp}/lnp64-userland-root"
out="${TMPDIR:-/tmp}/lnp64-userland.out"
expected="${TMPDIR:-/tmp}/lnp64-userland.expected"

rm -rf "$root"
mkdir -p "$root/bin" "$root/dev" "$root/etc" "$root/sbin" "$root/tmp"

cat > "$root/etc/motd" <<'MOTD'
welcome to lnp64 userland
MOTD

cat > "$root/etc/files" <<'FILES'
/bin/lnpsh.s
/bin/ucat.s
/dev/console
/dev/null
/dev/random
/etc/files
/etc/motd
/sbin/init.s
/tmp
FILES

cat > "$root/dev/devices" <<'DEVS'
console
null
random
DEVS

: > "$root/dev/console"
: > "$root/dev/null"
: > "$root/dev/random"

"${lnp64[@]}" cc userland/lnpsh.c -o "$root/bin/lnpsh.s"
"${lnp64[@]}" cc userland/ucat.c -o "$root/bin/ucat.s"
"${lnp64[@]}" cc userland/init.c -o "$root/sbin/init.s"

"${lnp64[@]}" run "$root/sbin/init.s" -- init "$root" > "$out"

cat > "$expected" <<EXPECTED
lnp64 init: boot
lnp64 init: root $root
welcome to lnp64 userland
lnpsh: scripted console
$ pwd
/
$ ls
/bin/lnpsh.s
/bin/ucat.s
/dev/console
/dev/null
/dev/random
/etc/files
/etc/motd
/sbin/init.s
/tmp
$ cat /etc/motd
welcome to lnp64 userland
$ ucat /etc/motd
welcome to lnp64 userland
$ devs
console
null
random
lnpsh: halt
EXPECTED

diff -u "$expected" "$out"
cat "$out"

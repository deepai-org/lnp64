#!/usr/bin/env bash
set -euo pipefail

mode="llvm"
usage() {
  cat <<'USAGE'
usage: scripts/run_userland.sh [--backend llvm] [--legacy-toy]

The default llvm backend runs the Clang/lld/run-elf userland probes through
the real LLVM package gate. --legacy-toy preserves the old host-directory
fork/exec smoke that compiles C with lnp64 cc --toy-bootstrap.
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
      if [[ "$mode" == "toy" ]]; then
        printf '%s\n' "toy backend is legacy-only; use --legacy-toy" >&2
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
    printf '%s\n' "== real LLVM LNP64 package gate: userland =="
    LNP64_LLVM_PACKAGE_FILTER=userland bash scripts/run_real_llvm_package_gate.sh
    printf '%s\n' "userland ok"
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

"${lnp64[@]}" cc --toy-bootstrap userland/lnpsh.c -o "$root/bin/lnpsh.s"
"${lnp64[@]}" cc --toy-bootstrap userland/ucat.c -o "$root/bin/ucat.s"
"${lnp64[@]}" cc --toy-bootstrap userland/init.c -o "$root/sbin/init.s"

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

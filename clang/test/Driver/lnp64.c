// NOTE: Scaffolded LNP64 driver acceptance test. This becomes executable once
// the Clang target and driver hook are wired into upstream dispatch tables.
// RUN: %clang -### --target=lnp64-unknown-none -ffreestanding -fno-pic -c %s 2>&1 | FileCheck %s --check-prefix=CC1
// RUN: %clang -### --target=lnp64-unknown-none -static %s 2>&1 | FileCheck %s --check-prefix=LINK
// XFAIL: *

int main(void) { return 0; }

// CC1: "-triple" "lnp64-unknown-none"
// CC1: "-ffreestanding"
// CC1: "-fno-pic"
// CC1: "target/lnp64-sysroot/usr/include"

// LINK: "ld.lld"
// LINK: "-m" "elf64lnp64"
// LINK: "target/lnp64-sysroot/usr/lib/lnp64/lnp64_static.ld"
// LINK: "target/lnp64-sysroot/usr/lib/lnp64/crt0.o"

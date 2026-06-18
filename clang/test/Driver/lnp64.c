// NOTE: Scaffolded LNP64 driver acceptance test. This becomes executable once
// the Clang target and driver hook are wired into upstream dispatch tables.
// RUN: %clang -### --target=lnp64-unknown-none -ffreestanding -fno-pic -c %s 2>&1 | FileCheck %s --check-prefix=CC1
// RUN: %clang -### --target=lnp64-unknown-none -static %s 2>&1 | FileCheck %s --check-prefix=LINK
// XFAIL: *

int main(void) { return 0; }

// CC1: "-triple" "lnp64-unknown-none"
// CC1: "-ffreestanding"
// CC1: "-fno-pic"

// LINK: "ld.lld"
// LINK: "-m" "elf64lnp64"
// LINK: "toolchain/lnp64_static.ld"
// LINK: "toolchain/crt0_lnp64.s"

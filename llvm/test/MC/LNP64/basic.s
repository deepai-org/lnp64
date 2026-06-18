# NOTE: Scaffolded LNP64 MC acceptance test. This becomes executable once the
# target is integrated into a buildable llvm-project tree.
# RUN: llvm-mc -triple=lnp64-unknown-none -show-encoding %s | FileCheck %s
# XFAIL: *

  nop
  li r1, 42
  add r3, r1, r2
  ld r4, 16(r31)
  st r4, 24(r31)
  call 0
  ret

# CHECK: nop
# CHECK: li r1, 42
# CHECK: add r3, r1, r2
# CHECK: ld r4, 16(r31)
# CHECK: st r4, 24(r31)
# CHECK: call 0
# CHECK: ret

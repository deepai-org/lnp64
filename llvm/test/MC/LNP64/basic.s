# NOTE: Scaffolded LNP64 MC acceptance test. This becomes executable once the
# target is integrated into a buildable llvm-project tree.
# RUN: llvm-mc -triple=lnp64-unknown-none -show-encoding %s | FileCheck %s
# XFAIL: *

  nop
  yield
  li r1, 42
  add r3, r1, r2
  ld r4, 16(r31)
  ld.h r5, 18(r31)
  st r4, 24(r31)
  st.h r5, 26(r31)
  call 0
  call main
  exec r1, r2, r3
main:
  ret

# CHECK: nop
# CHECK: yield
# CHECK: li r1, 42
# CHECK: add r3, r1, r2
# CHECK: ld r4, 16(r31)
# CHECK: ld.h r5, 18(r31)
# CHECK: st r4, 24(r31)
# CHECK: st.h r5, 26(r31)
# CHECK: call 0
# CHECK: call main
# CHECK: exec r1, r2, r3
# CHECK: ret

# LNP64 crt0 startup stub v2.
# Contract source for the real LLVM/lld crt0 object used by static Clang-linked
# run-elf smokes. v2 ISA: fixed 64-bit instructions, r1 = ra.

.text
.globl _start
.type _start,@function
_start:
  li r7, 0x7000
  li r8, 0x100
  mul r7, r7, r8
  ld r1, 0(r7)
  li r2, 8
  add r2, r7, r2
  li r8, 8
  mul r3, r1, r8
  add r3, r3, r2
  add r3, r3, r8
  errno_set r0
  jal r1, main
  # v2 ABI: main returns its value in r2 (the return-value register); r1 is ra.
  exit r2

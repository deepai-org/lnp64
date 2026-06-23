# LNP64 crt0 startup stub v2.
# Contract source for the real LLVM/lld crt0 object used by static Clang-linked
# run-elf smokes. v2 ISA: fixed 64-bit instructions, r1 = ra.

.text
.globl _start
.type _start,@function
_start:
  li r7, 0x1900000   # r7 = arg page base (ARG_BASE)
  ld r2, 0(r7)       # r2 = argc
  addi r3, r7, 8     # r3 = &argv[0]
  li r8, 8
  mul r4, r2, r8     # r4 = argc * 8
  add r4, r4, r8     # r4 += 8 (skip null terminator)
  add r4, r4, r3     # r4 = &envp[0]
  errno_set r0
  jal r1, main       # r2=argc r3=argv r4=envp; r1 <- return address
  # v2 ABI: main returns its value in r2 (the return-value register).
  exit r2

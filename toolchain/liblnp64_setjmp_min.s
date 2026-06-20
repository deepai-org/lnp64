# LNP64 setjmp/longjmp shim v0.
# jmp_buf words:
# 0 thread/generation cookie reserved
# 1 process/image cookie reserved
# 2 stack-bounds cookie reserved
# 3 saved r31 stack pointer
# 4 saved LR

.text
.globl setjmp
.type setjmp,@function
setjmp:
  ST r0, 0(r1)
  ST r0, 8(r1)
  ST r0, 16(r1)
  ST r31, 24(r1)
  LR_GET r2
  ST r2, 32(r1)
  LI r1, 0
  RET

.globl longjmp
.type longjmp,@function
longjmp:
  LD r3, 24(r1)
  LD r4, 32(r1)
  CMP r2, r0
  BNE longjmp_value_ready
  LI r2, 1
longjmp_value_ready:
  ADD r31, r3, r0
  LR_SET r4
  MOV r1, r2
  RET

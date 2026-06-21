# LNP64 setjmp/longjmp shim v0 (ISA v2).
# ABI: arg0 = r2 (jmp_buf ptr), arg1 = r3 (value), ret = r2, link = r1 (ra), sp = r31.
# jmp_buf words:
# 0 thread/generation cookie reserved
# 1 process/image cookie reserved
# 2 stack-bounds cookie reserved
# 3 saved r31 stack pointer
# 4 saved return address (r1/ra)

.text
.globl setjmp
.type setjmp,@function
setjmp:
  SD [r2, 0], r0
  SD [r2, 8], r0
  SD [r2, 16], r0
  SD [r2, 24], r31
  SD [r2, 32], r1
  LI r2, 0
  RET

.globl longjmp
.type longjmp,@function
longjmp:
  LD r4, [r2, 24]
  LD r5, [r2, 32]
  BNE r3, r0, longjmp_value_ready
  LI r3, 1
longjmp_value_ready:
  ADD r31, r4, r0
  MOV r1, r5
  MOV r2, r3
  RET

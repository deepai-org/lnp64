# LNP64 setjmp/longjmp shim v0 (ISA v2).
# ABI: arg0 = r2 (jmp_buf ptr), arg1 = r3 (value), ret = r2, link = r1 (ra), sp = r31.
# jmp_buf words:
#  0 thread/generation cookie reserved
#  1 process/image cookie reserved
#  2 stack-bounds cookie reserved
#  3 saved r31 stack pointer
#  4 saved return address (r1/ra)
#  5..14 saved callee-saved set s0..s9 = r18..r27
# longjmp must restore the caller's callee-saved s-registers (r18..r27): a
# function that called setjmp may have parked cross-call values there, and the
# unwinding longjmp returns control to that frame, so they must be reinstated.

.text
.globl setjmp
.type setjmp,@function
setjmp:
  SD r0, 0(r2)
  SD r0, 8(r2)
  SD r0, 16(r2)
  SD r31, 24(r2)
  SD r1, 32(r2)
  SD r18, 40(r2)
  SD r19, 48(r2)
  SD r20, 56(r2)
  SD r21, 64(r2)
  SD r22, 72(r2)
  SD r23, 80(r2)
  SD r24, 88(r2)
  SD r25, 96(r2)
  SD r26, 104(r2)
  SD r27, 112(r2)
  LI r2, 0
  RET

.globl longjmp
.type longjmp,@function
longjmp:
  LD r4, 24(r2)
  LD r5, 32(r2)
  LD r18, 40(r2)
  LD r19, 48(r2)
  LD r20, 56(r2)
  LD r21, 64(r2)
  LD r22, 72(r2)
  LD r23, 80(r2)
  LD r24, 88(r2)
  LD r25, 96(r2)
  LD r26, 104(r2)
  LD r27, 112(r2)
  BNE r3, r0, longjmp_value_ready
  LI r3, 1
longjmp_value_ready:
  ADD r31, r4, r0
  MOV r1, r5
  MOV r2, r3
  RET

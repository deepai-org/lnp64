.text
.globl _start
_start:
  li r29, -1

  li r20, 0x1234
  set_pcr r22, TP, r20
  sub r3, r22, r0
  get_pcr r1, TP
  sub r4, r1, r20
  or r3, r3, r4

  li r21, 0x55
  set_pcr r23, SIGMASK, r21
  sub r4, r23, r0
  or r3, r3, r4
  get_pcr r2, SIGMASK
  sub r4, r2, r21
  or r3, r3, r4

  set_pcr r24, PID, r20
  sub r4, r24, r29
  or r3, r3, r4

  set_pcr r25, CRED_PROFILE, r20
  sub r4, r25, r29
  or r3, r3, r4
  set_pcr r26, CRED_HANDLE, r20
  sub r4, r26, r29
  or r3, r3, r4

  exit r3

.text
.globl _start
_start:
  li r20, 1

  get_pcr r1, PID
  sub r3, r1, r20

  get_pcr r2, TID
  sub r4, r2, r20
  or r3, r3, r4

  get_pcr r5, TP
  or r3, r3, r5

  get_pcr r6, UID
  or r3, r3, r6

  get_pcr r7, GID
  or r3, r3, r7

  exit r3

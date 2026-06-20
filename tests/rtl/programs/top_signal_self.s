.data
sig_flag: .quad 0

.text
  LI r1, handler
  LI r2, 10
  SIGACTION r2, r1
  GET_PCR r3, PID
  KILL r3, r2
  LD r4, sig_flag
  LI r5, 1
  CMP r4, r5
  BNE bad
  EXIT r0

handler:
  LI r7, sig_flag
  LI r6, 1
  ST [r7, 0], r6
  SIGRET

bad:
  LI r1, 1
  EXIT r1

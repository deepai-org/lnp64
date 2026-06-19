.text
  LI r20, 0x1234
  SET_PCR TP, r20
  GET_PCR r1, TP
  CMP r1, r20
  BNE bad

  LI r21, 0x55
  SET_PCR SIGMASK, r21
  GET_PCR r2, SIGMASK
  CMP r2, r21
  BNE bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

.text
  LI r29, -1

  LI r20, 0x1234
  SET_PCR r22, TP, r20
  CMP r22, r0
  BNE bad
  GET_PCR r1, TP
  CMP r1, r20
  BNE bad

  LI r21, 0x55
  SET_PCR r23, SIGMASK, r21
  CMP r23, r0
  BNE bad
  GET_PCR r2, SIGMASK
  CMP r2, r21
  BNE bad

  SET_PCR r24, PID, r20
  CMP r24, r29
  BNE bad
  LI r28, 77
  ERRNO_SET r28
  SET_PCR r24, TID, r20
  CMP r24, r29
  BNE bad
  ERRNO_GET r27
  CMP r27, r28
  BNE bad

  SET_PCR r25, CRED_PROFILE, r20
  CMP r25, r29
  BNE bad
  ERRNO_GET r27
  CMP r27, r28
  BNE bad
  SET_PCR r26, CRED_HANDLE, r20
  CMP r26, r29
  BNE bad
  ERRNO_GET r27
  CMP r27, r28
  BNE bad
  SET_PCR r26, REALTIME_SEC, r20
  CMP r26, r29
  BNE bad
  ERRNO_GET r27
  CMP r27, r28
  BNE bad
  SET_PCR r26, REALTIME_NSEC, r20
  CMP r26, r29
  BNE bad
  ERRNO_GET r27
  CMP r27, r28
  BNE bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

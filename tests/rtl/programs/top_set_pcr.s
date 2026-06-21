.text
  LI r29, -1

  LI r20, 0x1234
  SET_PCR r22, TP, r20
  BNE r22, r0, bad
  GET_PCR r1, TP
  BNE r1, r20, bad

  LI r21, 0x55
  SET_PCR r23, SIGMASK, r21
  BNE r23, r0, bad
  GET_PCR r2, SIGMASK
  BNE r2, r21, bad

  GET_PCR r18, PID
  SET_PCR r24, PID, r20
  BNE r24, r29, bad
  GET_PCR r19, PID
  BNE r19, r18, bad
  LI r28, 77
  ERRNO_SET r28
  GET_PCR r18, TID
  SET_PCR r24, TID, r20
  BNE r24, r29, bad
  GET_PCR r19, TID
  BNE r19, r18, bad
  ERRNO_GET r27
  BNE r27, r28, bad

  GET_PCR r18, CRED_PROFILE
  SET_PCR r25, CRED_PROFILE, r20
  BNE r25, r29, bad
  GET_PCR r19, CRED_PROFILE
  BNE r19, r18, bad
  ERRNO_GET r27
  BNE r27, r28, bad
  GET_PCR r18, CRED_HANDLE
  SET_PCR r26, CRED_HANDLE, r20
  BNE r26, r29, bad
  GET_PCR r19, CRED_HANDLE
  BNE r19, r18, bad
  ERRNO_GET r27
  BNE r27, r28, bad
  SET_PCR r26, REALTIME_SEC, r20
  BNE r26, r29, bad
  ERRNO_GET r27
  BNE r27, r28, bad
  SET_PCR r26, REALTIME_NSEC, r20
  BNE r26, r29, bad
  ERRNO_GET r27
  BNE r27, r28, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

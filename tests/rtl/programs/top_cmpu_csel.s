.text
  LI r1, 5
  LI r2, 9
  LI r3, 1
  LI r4, 2
  LI r5, 4
  LI r6, 8
  CMP r1, r2
  CSEL.LT r7, r3, r4
  CSEL.GT r8, r3, r4
  CSEL.LE r9, r5, r6
  CSEL.GE r10, r5, r6
  LI r11, 16
  LI r12, 32
  CMP r1, r1
  CSEL.EQ r13, r11, r12
  CSEL.NE r14, r11, r12
  LI r15, -1
  LI r16, 1
  CMPU r15, r16
  LI r17, 64
  LI r18, 128
  LI r19, 256
  LI r21, 512
  CSEL.ULT r22, r17, r18
  CSEL.UGT r23, r17, r18
  CSEL.ULE r24, r19, r21
  CSEL.UGE r25, r19, r21
  ADD r26, r7, r8
  ADD r26, r26, r9
  ADD r26, r26, r10
  ADD r26, r26, r13
  ADD r26, r26, r14
  ADD r26, r26, r22
  ADD r26, r26, r23
  ADD r26, r26, r24
  ADD r26, r26, r25
  EXIT r26

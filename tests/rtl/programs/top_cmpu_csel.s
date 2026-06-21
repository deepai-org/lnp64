.text
  LI r1, 5
  LI r2, 9
  LI r3, 1
  LI r4, 2
  LI r5, 4
  LI r6, 8
  ; v2: CMP r1,r2 + CSEL.<cc> rd, rT, rF -> rd = cond ? rT : rF
  ; select idiom: SUB r29,rT,rF; MUL r29,cond,r29; ADD rd,rF,r29
  SLT r28, r1, r2          ; cond = (r1 <s r2)      (CSEL.LT r7,r3,r4)
  SUB r29, r3, r4
  MUL r29, r28, r29
  ADD r7, r4, r29
  SLT r28, r2, r1          ; cond = (r1 >s r2)      (CSEL.GT r8,r3,r4)
  SUB r29, r3, r4
  MUL r29, r28, r29
  ADD r8, r4, r29
  SLT r30, r2, r1
  SLTIU r28, r30, 1        ; cond = (r1 <=s r2)     (CSEL.LE r9,r5,r6)
  SUB r29, r5, r6
  MUL r29, r28, r29
  ADD r9, r6, r29
  SLT r30, r1, r2
  SLTIU r28, r30, 1        ; cond = (r1 >=s r2)     (CSEL.GE r10,r5,r6)
  SUB r29, r5, r6
  MUL r29, r28, r29
  ADD r10, r6, r29
  LI r11, 16
  LI r12, 32
  SUB r30, r1, r1
  SLTIU r28, r30, 1        ; cond = (r1 == r1)      (CSEL.EQ r13,r11,r12)
  SUB r29, r11, r12
  MUL r29, r28, r29
  ADD r13, r12, r29
  SUB r30, r1, r1
  SLTU r28, r0, r30        ; cond = (r1 != r1)      (CSEL.NE r14,r11,r12)
  SUB r29, r11, r12
  MUL r29, r28, r29
  ADD r14, r12, r29
  LI r15, -1
  LI r16, 1
  LI r17, 64
  LI r18, 128
  LI r19, 256
  LI r21, 512
  SLTU r28, r15, r16       ; cond = (r15 <u r16)    (CSEL.ULT r22,r17,r18)
  SUB r29, r17, r18
  MUL r29, r28, r29
  ADD r22, r18, r29
  SLTU r28, r16, r15       ; cond = (r15 >u r16)    (CSEL.UGT r23,r17,r18)
  SUB r29, r17, r18
  MUL r29, r28, r29
  ADD r23, r18, r29
  SLTU r30, r16, r15
  SLTIU r28, r30, 1        ; cond = (r15 <=u r16)   (CSEL.ULE r24,r19,r21)
  SUB r29, r19, r21
  MUL r29, r28, r29
  ADD r24, r21, r29
  SLTU r30, r15, r16
  SLTIU r28, r30, 1        ; cond = (r15 >=u r16)   (CSEL.UGE r25,r19,r21)
  SUB r29, r19, r21
  MUL r29, r28, r29
  ADD r25, r21, r29
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

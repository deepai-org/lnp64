.text
  LI r1, 5
  LI r2, 9
  ; v2: CMP r1,r2 + CSET.<cc> -> slt/sltu idioms (a=r1, b=r2)
  SLT r3, r1, r2          ; r3 = (r1 <s r2)         (was CSET.LT)
  SLT r4, r2, r1          ; r4 = (r1 >s r2)         (was CSET.GT)
  SLT r28, r2, r1         ; t = (r1 >s r2)
  SLTIU r5, r28, 1        ; r5 = (r1 <=s r2)        (was CSET.LE)
  SLT r28, r1, r2         ; t = (r1 <s r2)
  SLTIU r6, r28, 1        ; r6 = (r1 >=s r2)        (was CSET.GE)
  ; CMP r1,r1 (equal)
  SUB r28, r1, r1
  SLTIU r7, r28, 1        ; r7 = (r1 == r1)         (was CSET.EQ)
  SUB r28, r1, r1
  SLTU r8, r0, r28        ; r8 = (r1 != r1)         (was CSET.NE)
  LI r15, -1
  LI r16, 1
  ; CMPU r15,r16 (unsigned: r15 huge, r16=1)
  SLTU r9, r15, r16       ; r9 = (r15 <u r16)       (was CSET.ULT)
  SLTU r10, r16, r15      ; r10 = (r15 >u r16)      (was CSET.UGT)
  SLTU r28, r16, r15      ; t = (r15 >u r16)
  SLTIU r11, r28, 1       ; r11 = (r15 <=u r16)     (was CSET.ULE)
  SLTU r28, r15, r16      ; t = (r15 <u r16)
  SLTIU r12, r28, 1       ; r12 = (r15 >=u r16)     (was CSET.UGE)
  ADD r13, r3, r4
  ADD r13, r13, r5
  ADD r13, r13, r6
  ADD r13, r13, r7
  ADD r13, r13, r8
  ADD r13, r13, r9
  ADD r13, r13, r10
  ADD r13, r13, r11
  ADD r13, r13, r12
  EXIT r13

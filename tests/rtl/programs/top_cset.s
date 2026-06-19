.text
  LI r1, 5
  LI r2, 9
  CMP r1, r2
  CSET.LT r3
  CSET.GT r4
  CSET.LE r5
  CSET.GE r6
  CMP r1, r1
  CSET.EQ r7
  CSET.NE r8
  LI r15, -1
  LI r16, 1
  CMPU r15, r16
  CSET.ULT r9
  CSET.UGT r10
  CSET.ULE r11
  CSET.UGE r12
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

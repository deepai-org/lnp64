.text
  AUIPC r3, 0
  FENCE.SC
  AUIPC r4, 8
  LI r1, 1
  LI r2, 0
  LI r5, 4096
  CMP r3, r5
  CSEL.EQ r6, r1, r2
  LI r7, 4116
  CMP r4, r7
  CSEL.EQ r8, r1, r2
  ADD r9, r6, r8
  EXIT r9

.text
  AUIPC r3, 0
  FENCE.SC
  AUIPC r4, 8
  LI r1, 1
  LI r2, 0
  LI r5, 4096
  ; v2: r6 = (r3 == r5) ? 1 : 0
  SUB r28, r3, r5
  SLTIU r6, r28, 1
  LI r7, 4116
  ; v2: r8 = (r4 == r7) ? 1 : 0
  SUB r28, r4, r7
  SLTIU r8, r28, 1
  ADD r9, r6, r8
  EXIT r9

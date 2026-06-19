.text
  LI r1, 0
  LI r2, 0x12345678
  ST.W [r1, 0], r2
  LD.W r3, [r1, 0]
  LI r4, 0xabcd
  ST.H [r1, 4], r4
  LD.H r5, [r1, 4]
  LI r6, 0x55aa
  ST.H [r1, 6], r6
  LD.W r7, [r1, 4]
  LI r8, 0x12345678
  LI r9, 0xabcd
  LI r10, 0x55aaabcd
  LI r11, 1
  LI r12, 0
  CMP r3, r8
  CSEL.EQ r13, r11, r12
  CMP r5, r9
  CSEL.EQ r14, r11, r12
  ADD r13, r13, r14
  CMP r7, r10
  CSEL.EQ r15, r11, r12
  ADD r13, r13, r15
  EXIT r13

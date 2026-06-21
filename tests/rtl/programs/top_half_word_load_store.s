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
  ; v2: r13 = (r3 == r8) ? 1 : 0
  SUB r28, r3, r8
  SLTIU r13, r28, 1
  ; v2: r14 = (r5 == r9) ? 1 : 0
  SUB r28, r5, r9
  SLTIU r14, r28, 1
  ADD r13, r13, r14
  ; v2: r15 = (r7 == r10) ? 1 : 0
  SUB r28, r7, r10
  SLTIU r15, r28, 1
  ADD r13, r13, r15
  EXIT r13

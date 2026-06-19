.text
  LI r1, 1
  LSLI r1, r1, 32
  MOV r2, r1
  MULH r3, r1, r2
  MULHU r4, r1, r2
  LI r5, -1
  LSLI r5, r5, 32
  MULH r6, r5, r1
  MULHSU r7, r5, r1
  MULHU r8, r5, r1
  LI r9, 1
  LI r10, -1
  LI r11, 0xffffffff
  LI r12, 0
  CMP r3, r9
  CSEL.EQ r13, r9, r12
  CMP r4, r9
  CSEL.EQ r14, r9, r12
  ADD r13, r13, r14
  CMP r6, r10
  CSEL.EQ r15, r9, r12
  ADD r13, r13, r15
  CMP r7, r10
  CSEL.EQ r16, r9, r12
  ADD r13, r13, r16
  CMP r8, r11
  CSEL.EQ r17, r9, r12
  ADD r13, r13, r17
  EXIT r13

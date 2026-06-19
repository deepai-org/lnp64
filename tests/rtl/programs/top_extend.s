.text
  LI r1, 255
  SEXT.B r2, r1
  ADDI r2, r2, 2
  ZEXT.B r3, r1
  LI r4, 65535
  SEXT.H r5, r4
  ADDI r5, r5, 3
  ZEXT.H r6, r4
  LI r7, 4294967295
  SEXT.W r8, r7
  ADDI r8, r8, 4
  ZEXT.W r9, r7
  ADD r10, r2, r5
  ADD r10, r10, r8
  ADDI r10, r10, 6
  EXIT r10

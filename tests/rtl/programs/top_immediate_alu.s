.text
  LI r1, 8
  ADDI r2, r1, 5
  ANDI r3, r2, 15
  ORI r4, r3, 32
  XORI r5, r4, 7
  LSLI r6, r5, 1
  LSRI r7, r6, 2
  LI r8, -8
  ASRI r9, r8, 1
  ADDI r10, r9, 11
  ADD r11, r7, r10
  EXIT r11

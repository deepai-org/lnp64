.text
  LI r1, 16
  CLZ r2, r1
  ADDI r2, r2, -55
  CTZ r3, r1
  LI r4, 3855
  POPCNT r4, r4
  ADDI r4, r4, -5
  LI r5, 1
  LI r6, 8
  ROL r7, r5, r6
  ROR r8, r7, r6
  LI r9, 4660
  BSWAP16 r10, r9
  ANDI r10, r10, 15
  LI r11, 305419896
  BSWAP32 r11, r11
  LSRI r11, r11, 24
  ANDI r11, r11, 15
  LI r12, 255
  BSWAP64 r12, r12
  LSRI r12, r12, 56
  ANDI r12, r12, 15
  ADD r13, r2, r3
  ADD r13, r13, r4
  ADD r13, r13, r8
  ADD r13, r13, r10
  ADD r13, r13, r11
  ADD r13, r13, r12
  EXIT r13

.text
  LI r1, 0
  LI r2, 5
  ST [r1, 0], r2
  LI r3, 7
  AMO.ADD r4, r1, r3
  AMO.XOR r5, r1, r2
  AMO.SWAP r6, r1, r3
  AMO.AND r7, r1, r2
  AMO.OR r8, r1, r3
  LD r9, [r1, 0]
  ADD r10, r4, r5
  ADD r10, r10, r6
  ADD r10, r10, r7
  ADD r10, r10, r8
  ADD r10, r10, r9
  EXIT r10

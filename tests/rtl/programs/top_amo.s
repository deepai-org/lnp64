.text
  LI r1, 0
  LI r2, 5
  ST [r1, 0], r2
  LI r3, 7
  ; v2: AMO.ADD r4, r1, r3  ->  LR/SC loop (r4 = old, mem += r3)
amo_add:
  LR.D r4, r1
  ADD r29, r4, r3
  SC.D r30, r29, r1
  BNE r30, r0, amo_add
  ; v2: AMO.XOR r5, r1, r2
amo_xor:
  LR.D r5, r1
  XOR r29, r5, r2
  SC.D r30, r29, r1
  BNE r30, r0, amo_xor
  ; v2: AMO.SWAP r6, r1, r3
amo_swap:
  LR.D r6, r1
  SC.D r30, r3, r1
  BNE r30, r0, amo_swap
  ; v2: AMO.AND r7, r1, r2
amo_and:
  LR.D r7, r1
  AND r29, r7, r2
  SC.D r30, r29, r1
  BNE r30, r0, amo_and
  ; v2: AMO.OR r8, r1, r3
amo_or:
  LR.D r8, r1
  OR r29, r8, r3
  SC.D r30, r29, r1
  BNE r30, r0, amo_or
  LD r9, [r1, 0]
  ADD r10, r4, r5
  ADD r10, r10, r6
  ADD r10, r10, r7
  ADD r10, r10, r8
  ADD r10, r10, r9
  EXIT r10

.text
  LI r1, 5
  LI r2, 7
  SEL.LT r3, r1, r2, r1, r2
  SEL.EQ r4, r1, r2, r1, r2
  SEL.GEU r5, r2, r1, r1, r2
  ADD r6, r3, r4
  ADD r6, r6, r5
  EXIT r6

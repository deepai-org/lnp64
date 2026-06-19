.text
  LI r1, 7
  LI r2, 5
  ADD r3, r1, r2
  ST [r0, 0], r3
  LD r4, [r0, 0]
  JMP after_skip
  LI r5, 99
after_skip:
  LI r10, 2
  ENV_GET r6, r10, r0, r0
  EXIT r4

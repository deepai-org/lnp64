.data
ok_msg: .string "guarded heap overflow ok\n"
dma: .zero 40

.text
  LI r29, -1
  LI r1, 32
  LI r2, 64
  ALLOC_EX r3, r1, r2
  CMP r3, r29
  BEQ bad

in_bounds_write:
  LI r4, 123
  ST [r3, 0], r4
  LD r5, [r3, 0]
  CMP r5, r4
  BNE bad

guarded_overflow_dma:
  LI r10, dma
  LI r1, 2
  ST [r10, 0], r1
  LI r1, 32
  ADD r6, r3, r1
  ST [r10, 8], r6
  LI r1, 238
  ST [r10, 16], r1
  LI r1, 1
  ST [r10, 24], r1
  ST [r10, 32], r0
  DMA_CTL r7, r10
  CMP r7, r29
  BNE bad
  ERRNO_GET r8
  LI r1, 14
  CMP r8, r1
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 25
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

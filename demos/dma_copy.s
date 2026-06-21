.data
ok_msg: .string "dma copy ok\n"
obj: .zero 64

.text
  LI r29, -1
  LI r1, 32
  ALLOC r3, r1
  BEQ r3, r29, bad

create_dma_buffer:
  LI r10, obj
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 4
  ST [r10, 8], r1
  ST [r10, 16], r0
  ST [r10, 24], r0
  ST [r10, 32], r0
  ST [r10, 40], r3
  LI r1, 32
  ST [r10, 48], r1
  OBJECT_CTL r4, r10
  BEQ r4, r29, bad

copy_within_buffer:
  LI r5, 90
  ST [r3, 0], r5
  LI r6, 8
  ADD r7, r3, r6
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r7
  ST [r10, 16], r3
  ST [r10, 24], r6
  ST [r10, 32], r4
  DMA_CTL r8, r10
  BNE r8, r6, bad
  LD r9, [r7, 0]
  BNE r9, r5, bad

reject_out_of_scope_destination:
  LI r1, 40
  ADD r7, r3, r1
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r7
  ST [r10, 16], r3
  ST [r10, 24], r6
  ST [r10, 32], r4
  DMA_CTL r8, r10
  BNE r8, r29, bad
  ERRNO_GET r11
  LI r1, 14
  BNE r11, r1, bad

done:
  LI r1, ok_msg
  LI r2, 12
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

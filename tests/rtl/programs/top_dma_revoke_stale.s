.data
obj: .zero 64

.text
  LI r29, -1
  LI r1, 16
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
  LI r1, 16
  ST [r10, 48], r1
  OBJECT_CTL r4, r10
  BEQ r4, r29, bad

fill_before_revoke:
  LI r1, 2
  ST [r10, 0], r1
  ST [r10, 8], r3
  LI r1, 170
  ST [r10, 16], r1
  LI r1, 1
  ST [r10, 24], r1
  ST [r10, 32], r4
  DMA_CTL r5, r10
  LI r1, 1
  BNE r5, r1, bad
  LD.B r11, [r3, 0]
  LI r1, 170
  BNE r11, r1, bad

revoke_buffer:
  ST [r10, 0], r4
  CAP_REVOKE r6, r10
  BLE r6, r0, bad

fill_after_revoke_rejected:
  LI r1, 2
  ST [r10, 0], r1
  ST [r10, 8], r3
  LI r1, 187
  ST [r10, 16], r1
  LI r1, 1
  ST [r10, 24], r1
  ST [r10, 32], r4
  DMA_CTL r7, r10
  BNE r7, r29, bad
  ERRNO_GET r8
  LI r1, 116
  BNE r8, r1, bad
  LD.B r12, [r3, 0]
  LI r1, 170
  BNE r12, r1, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

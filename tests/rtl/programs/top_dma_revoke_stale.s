.data
obj: .zero 64

.text
  LI r29, -1
  LI r1, 16
  ALLOC r3, r1
  CMP r3, r29
  BEQ bad

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
  CMP r4, r29
  BEQ bad

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
  CMP r5, r1
  BNE bad
  LD.B r11, [r3, 0]
  LI r1, 170
  CMP r11, r1
  BNE bad

revoke_buffer:
  ST [r10, 0], r4
  CAP_REVOKE r6, r10
  CMP r6, r0
  BLE bad

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
  CMP r7, r29
  BNE bad
  ERRNO_GET r8
  LI r1, 116
  CMP r8, r1
  BNE bad
  LD.B r12, [r3, 0]
  LI r1, 170
  CMP r12, r1
  BNE bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

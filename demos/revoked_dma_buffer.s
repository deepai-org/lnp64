.data
_epdesc: .zero 32
ok_msg: .string "revoked dma buffer ok\n"
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

dma_before_revoke:
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

revoke_buffer:
  ST [r10, 0], r4
  CAP_REVOKE r6, r10
  BLE r6, r0, bad

dma_after_revoke:
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

done:
  LI r1, ok_msg
  LI r2, 22
  LI r25, 1
  LI r24, _epdesc
  ST [r24, 0], r1
  ST [r24, 8], r2
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd1 -> send over byte-fd
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

.data
ok_msg: .string "allocator native ok\n"

.text
  LI r29, -1

plain_alloc_size:
  LI r1, 24
  ALLOC r3, r1
  CMP r3, r29
  BEQ bad
  ALLOC_SIZE r4, r3
  CMP r4, r1
  BNE bad
  LI r5, 171
  ST [r3, 16], r5
  LD r6, [r3, 16]
  CMP r6, r5
  BNE bad
  FREE r3
  ALLOC_SIZE r4, r3
  CMP r4, r0
  BNE bad

guarded_aligned_alloc:
  LI r1, 32
  LI r2, 128
  ALLOC_EX r7, r1, r2
  CMP r7, r29
  BEQ bad
  ALLOC_SIZE r8, r7
  CMP r8, r1
  BNE bad
  LI r9, 127
  AND r10, r7, r9
  CMP r10, r0
  BNE bad
  LI r11, 205
  ST [r7, 24], r11
  LD r12, [r7, 24]
  CMP r12, r11
  BNE bad
  FREE r7
  ALLOC_SIZE r8, r7
  CMP r8, r0
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 20
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

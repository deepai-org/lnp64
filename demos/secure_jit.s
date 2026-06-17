.data
ok_msg: .string "secure jit ok\n"

.text
  LI r29, -1

map_rw:
  LI r1, 4096
  LI r2, 3
  MMAP r3, r0, r1, r2, fd0, r0
  CMP r3, r29
  BEQ bad
  LI r4, 144
  ST [r3, 0], r4

protect_rx:
  LI r5, 5
  MPROTECT r3, r1, r5
  ERRNO_GET r6
  CMP r6, r0
  BNE bad

reject_wx:
  LI r5, 6
  MPROTECT r3, r1, r5
  ERRNO_GET r6
  LI r7, 1
  CMP r6, r7
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 14
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

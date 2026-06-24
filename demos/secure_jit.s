.data
_epdesc: .zero 32
ok_msg: .string "secure jit ok\n"

.text
  LI r29, -1

map_rw:
  LI r9, 4096
  LI r2, 3
  MMAP r3, r0, r9, r2, fd0, r0
  BEQ r3, r29, bad
  LI r4, 144
  ST [r3, 0], r4

protect_rx:
  LI r5, 5
  MPROTECT r3, r9, r5
  ERRNO_GET r6
  BNE r6, r0, bad
  ISYNC r8, r3, r9
  BNE r8, r0, bad

reject_wx:
  LI r5, 6
  MPROTECT r3, r9, r5
  ERRNO_GET r6
  LI r7, 1
  BNE r6, r7, bad

done:
  LI r1, ok_msg
  LI r2, 14
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

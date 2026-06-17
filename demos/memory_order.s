.data
ok_msg: .string "memory order ok\n"
word: .quad 41

.text
  LI r4, word
  LI r5, 8

isync_forms:
  ISYNC r4, r5
  CMP r1, r0
  BNE bad
  ISYNC r11, r4, r5
  CMP r11, r0
  BNE bad

successful_cmpxchg:
  FENCE
  LI r6, 41
  LI r7, 42
  LOCK.CMPXCHG r8, r4, r6, r7
  CMP r8, r6
  BNE bad
  LD r9, [r4, 0]
  CMP r9, r7
  BNE bad

failed_cmpxchg:
  FENCE
  LI r6, 41
  LI r7, 99
  LOCK.CMPXCHG r8, r4, r6, r7
  LI r10, 42
  CMP r8, r10
  BNE bad
  LD r9, [r4, 0]
  CMP r9, r10
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 16
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

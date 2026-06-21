.data
word: .quad 41
ok_msg: .string "memory order ok\n"

.text
  LI r4, word
  LI r5, 8

isync_forms:
  ISYNC r4, r5
  BNE r1, r0, bad
  ISYNC r11, r4, r5
  BNE r11, r0, bad

successful_cmpxchg:
  FENCE
  LI r6, 41
  LI r7, 42
succ_retry:
  LR.D r8, r4
  BNE r8, r6, succ_done
  SC.D r12, r7, r4
  BNE r12, r0, succ_retry
succ_done:
  BNE r8, r6, bad
  LD r9, [r4, 0]
  BNE r9, r7, bad

failed_cmpxchg:
  FENCE
  LI r6, 41
  LI r7, 99
fail_retry:
  LR.D r8, r4
  BNE r8, r6, fail_done
  SC.D r12, r7, r4
  BNE r12, r0, fail_retry
fail_done:
  LI r10, 42
  BNE r8, r10, bad
  LD r9, [r4, 0]
  BNE r9, r10, bad

done:
  LI r1, ok_msg
  LI r2, 16
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

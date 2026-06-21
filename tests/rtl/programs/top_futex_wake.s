.data
futex_word: .quad 0
child_result: .quad 0

.text
  LI r14, futex_word
  LI r15, 0
  ST [r14, 0], r15
  LI r16, waiter
  CLONE.SPAWN r17, r16, r0
  YIELD
  LI r18, 1
  ST [r14, 0], r18
  FUTEX_WAKE r14, r18
  LI r23, child_result
  THREAD_JOIN r22, r17, r23
  LD r19, [r14, 0]
  LI r21, 2
  BNE r19, r21, bad
  LD r24, [r23, 0]
  BNE r24, r0, bad
  EXIT r0

waiter:
  FUTEX_WAIT r14, r15
  LI r18, 2
  ST [r14, 0], r18
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

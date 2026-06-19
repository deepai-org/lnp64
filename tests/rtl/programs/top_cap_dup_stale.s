.data
cap_arg: .zero 32

.text
  LI r29, -1
  LI r10, cap_arg

dup_stdout_to_fd4:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 4
  ST [r10, 8], r1
  ST [r10, 16], r0
  ST [r10, 24], r0
  CAP_DUP r11, r10
  CMP r11, r29
  BEQ bad

bump_fd4_generation:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 4
  ST [r10, 8], r1
  ST [r10, 16], r0
  ST [r10, 24], r0
  CAP_DUP r12, r10
  CMP r12, r29
  BEQ bad
  CMP r12, r11
  BEQ bad

old_token_rejected:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  ST [r10, 16], r0
  ST [r10, 24], r0
  CAP_DUP r13, r10
  CMP r13, r29
  BNE bad
  ERRNO_GET r14
  LI r1, 116
  CMP r14, r1
  BNE bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

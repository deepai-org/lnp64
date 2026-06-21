.data
cap_arg: .zero 32

.text
  LI r29, -1
  LI r10, cap_arg

create_read_dup_child:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 4
  ST [r10, 8], r1
  LI r1, 65
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r11, r10
  BEQ r11, r29, bad

reject_broader_rights:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  LI r1, 3
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r12, r10
  BNE r12, r29, bad
  ERRNO_GET r13
  LI r1, 1
  BNE r13, r1, bad

allow_further_narrowing:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  LI r1, 1
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r14, r10
  BEQ r14, r29, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

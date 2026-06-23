.text
  LI r29, -1
  LI r10, 0

create_pipe_queue:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 2
  ST [r10, 8], r1
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 3
  ST [r10, 24], r1
  LI r1, 4
  ST [r10, 32], r1
  ST [r10, 40], r0
  OBJECT_CTL r11, r10
  BNE r11, r0, bad

push_byte:
  LI r12, 80
  LI r13, 1
  LI r1, 90
  ST.B [r12, 0], r1
  LI r4, 4
  PUSH r14, fd4, r12, r13
  BNE r14, r13, bad

pull_byte:
  LI r15, 88
  LI r3, 3
  PULL r16, fd3, r15, r13
  BNE r16, r13, bad
  LD.B r17, [r15, 0]
  LI r18, 90
  BNE r17, r18, bad
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

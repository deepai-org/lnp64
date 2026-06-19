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
  CMP r11, r0
  BNE bad

push_byte:
  LI r12, 80
  LI r13, 1
  LI r1, 90
  ST.B [r12, 0], r1
  PUSH r14, fd4, r12, r13
  CMP r14, r13
  BNE bad

pull_byte:
  LI r15, 88
  PULL r16, fd3, r15, r13
  CMP r16, r13
  BNE bad
  LD.B r17, [r15, 0]
  LI r18, 90
  CMP r17, r18
  BNE bad
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

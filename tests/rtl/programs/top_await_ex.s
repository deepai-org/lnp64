.data
arg: .quad 0
mask: .quad 1

.text
  LI r29, -1
  LI r10, 0
  LI r20, 1
  LI r22, arg

create_ready_event_counter:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r1
  ST [r10, 16], r1
  LI r1, 4
  ST [r10, 24], r1
  ST [r10, 32], r0
  LI r1, 1
  ST [r10, 40], r1
  ST [r10, 48], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad

probe_mode_ready:
  LI r4, 4
  ST [r22, 0], r0
  ST [r22, 8], r20
  AWAIT_EX r12, fd4, r22
  BNE r12, r20, bad

zero_timeout_dynamic_no_ready:
  LI r12, 80
  LI r13, 8
  READ_FD fd4, r12, r13
  BNE r1, r13, bad
  LI r1, 1
  ST [r22, 0], r1
  ST [r22, 8], r20
  LI r4, 4
  AWAIT_EX r17, fd4, r22
  BNE r17, r0, bad

invalid_mode:
  LI r4, 4
  LI r1, 99
  ST [r22, 0], r1
  ST [r22, 8], r20
  AWAIT_EX r18, fd4, r22
  LI r1, -22
  BNE r18, r1, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

.data
ok_msg: .string "call gate modes ok\n"
obj: .zero 72
counter_out: .zero 8

.text
  LI r29, -1
  LI r10, obj

create_completion_counter:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 1
  ST [r10, 8], r1
  ST [r10, 16], r0
  LI r1, 3
  ST [r10, 24], r1
  ST [r10, 32], r0
  ST [r10, 40], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad

create_async_gate:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 2
  ST [r10, 8], r1
  LI r1, 4
  ST [r10, 16], r1
  ST [r10, 24], r1
  ST [r10, 32], r0
  ST [r10, 40], r0
  LI r1, 1
  ST [r10, 48], r1
  LI r1, 3
  ST [r10, 56], r1
  ST [r10, 64], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad
  LI r12, 10
  LI r13, 20
  CALL_CAP r15, fd4, r12, r13
  BLE r15, r0, bad
  LI r16, counter_out
  LI r17, 8
  READ_FD fd3, r16, r17
  BNE r1, r17, bad
  LD r18, [r16, 0]
  BNE r18, r15, bad

create_handoff_gate:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 2
  ST [r10, 8], r1
  LI r1, 4
  ST [r10, 16], r1
  LI r1, 5
  ST [r10, 24], r1
  ST [r10, 32], r0
  LI r1, handoff_service
  ST [r10, 40], r1
  LI r1, 2
  ST [r10, 48], r1
  ST [r10, 56], r0
  ST [r10, 64], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad
  LI r12, 33
  LI r13, 44
  CALL_CAP r19, fd5, r12, r13
  JMP bad

handoff_service:
  LI r20, 33
  BNE r1, r20, bad
  LI r20, 44
  BNE r2, r20, bad
  BNE r19, r0, bad
  LI r1, ok_msg
  LI r2, 19
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

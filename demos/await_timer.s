.data
ok_msg: .string "await timer ok\n"
obj: .zero 72
timer_ticks: .quad 2
timer_out: .zero 8

.text
  LI r29, -1
  LI r10, obj
  LI r20, 1

create_timer:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 6
  ST [r10, 8], r1
  ST [r10, 16], r0
  LI r1, 3
  ST [r10, 24], r1
  ST [r10, 32], r0
  ST [r10, 40], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad
  LI r12, timer_ticks
  LI r13, 8
  WRITE_FD fd3, r12, r13

await_timer:
  AWAIT r14, fd3, r20
  BNE r14, r0, bad
  LI r12, timer_out
  READ_FD fd3, r12, r13
  LD r15, [r12, 0]
  BLE r15, r0, bad

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
  LI r16, 4
  AWAIT_DYN r17, r16, r20
  BEQ r17, r29, bad

done:
  LI r1, ok_msg
  LI r2, 15
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

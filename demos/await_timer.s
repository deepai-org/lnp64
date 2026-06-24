.data
_epdesc: .zero 32
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
  LI r25, 3
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd3 -> send over byte-fd

await_timer:
  AWAIT r14, fd3, r20
  BNE r14, r0, bad
  LI r12, timer_out
  LI r25, 3
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  RECV r26, r25, r24  # read_fd fd3 -> recv over byte-fd
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
  LI r25, 1
  LI r24, _epdesc
  ST [r24, 0], r1
  ST [r24, 8], r2
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd1 -> send over byte-fd
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

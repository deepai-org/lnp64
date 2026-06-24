.data
_epdesc: .zero 32
ok_msg: .string "object profiles ok\n"
obj: .zero 72
pipe_msg: .string "Q"
pipe_out: .zero 8
event_val: .quad 5
event_out: .zero 8
timer_ticks: .quad 2
timer_out: .zero 8
mem_msg: .string "M"

.text
  LI r29, -1
  LI r10, obj

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
  BEQ r11, r29, bad
  LI r12, pipe_msg
  LI r13, 1
  PUSH r14, fd4, r12, r13
  BNE r14, r13, bad
  LI r15, pipe_out
  PULL r14, fd3, r15, r13
  BNE r14, r13, bad
  LD.B r16, [r15, 0]
  LI r17, 81
  BNE r16, r17, bad

create_event_counter:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 1
  ST [r10, 8], r1
  ST [r10, 16], r1
  LI r1, 5
  ST [r10, 24], r1
  ST [r10, 32], r0
  ST [r10, 40], r0
  ST [r10, 48], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad
  LI r12, event_val
  LI r13, 8
  LI r25, 5
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd5 -> send over byte-fd
  ERRNO_GET r18
  BNE r18, r0, bad
  LI r12, event_out
  LI r25, 5
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  RECV r26, r25, r24  # read_fd fd5 -> recv over byte-fd
  BNE r1, r13, bad
  LD r19, [r12, 0]
  LI r20, 5
  BNE r19, r20, bad

create_timer:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 6
  ST [r10, 8], r1
  ST [r10, 16], r0
  LI r1, 6
  ST [r10, 24], r1
  ST [r10, 32], r0
  ST [r10, 40], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad
  LI r12, timer_ticks
  LI r13, 8
  LI r25, 6
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd6 -> send over byte-fd
  LI r21, 5
  SLEEP r21
  LI r12, timer_out
  LI r25, 6
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  RECV r26, r25, r24  # read_fd fd6 -> recv over byte-fd
  BNE r1, r13, bad
  LD r22, [r12, 0]
  BLE r22, r0, bad

create_memory_object:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 3
  ST [r10, 8], r1
  ST [r10, 16], r0
  LI r1, 7
  ST [r10, 24], r1
  ST [r10, 32], r0
  LI r1, 16
  ST [r10, 40], r1
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad
  LI r12, mem_msg
  LI r13, 1
  LI r25, 7
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd7 -> send over byte-fd
  BNE r1, r13, bad

create_call_gate:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 2
  ST [r10, 8], r1
  LI r1, 4
  ST [r10, 16], r1
  LI r1, 8
  ST [r10, 24], r1
  ST [r10, 32], r0
  LI r1, service
  ST [r10, 40], r1
  ST [r10, 48], r0
  ST [r10, 56], r0
  ST [r10, 64], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad
  LI r1, 7
  LI r2, 9
  CALL_CAP r23, fd8, r1, r2
  LI r24, 16
  BNE r23, r24, bad
  LI r24, 9
  BNE r30, r24, bad

done:
  LI r1, ok_msg
  LI r2, 19
  LI r25, 1
  LI r24, _epdesc
  ST [r24, 0], r1
  ST [r24, 8], r2
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd1 -> send over byte-fd
  EXIT r0

service:
  ADD r3, r1, r2
  RET_CAP r0, r3, r2

bad:
  LI r1, 1
  EXIT r1

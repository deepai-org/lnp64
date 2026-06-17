.data
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
  CMP r11, r29
  BEQ bad
  LI r12, pipe_msg
  LI r13, 1
  PUSH r14, fd4, r12, r13
  CMP r14, r13
  BNE bad
  LI r15, pipe_out
  PULL r14, fd3, r15, r13
  CMP r14, r13
  BNE bad
  LD.B r16, [r15, 0]
  LI r17, 81
  CMP r16, r17
  BNE bad

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
  CMP r11, r29
  BEQ bad
  LI r12, event_val
  LI r13, 8
  WRITE_FD fd5, r12, r13
  ERRNO_GET r18
  CMP r18, r0
  BNE bad
  LI r12, event_out
  READ_FD fd5, r12, r13
  CMP r1, r13
  BNE bad
  LD r19, [r12, 0]
  LI r20, 5
  CMP r19, r20
  BNE bad

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
  CMP r11, r29
  BEQ bad
  LI r12, timer_ticks
  LI r13, 8
  WRITE_FD fd6, r12, r13
  LI r21, 5
  SLEEP r21
  LI r12, timer_out
  READ_FD fd6, r12, r13
  CMP r1, r13
  BNE bad
  LD r22, [r12, 0]
  CMP r22, r0
  BLE bad

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
  CMP r11, r29
  BEQ bad
  LI r12, mem_msg
  LI r13, 1
  WRITE_FD fd7, r12, r13
  CMP r1, r13
  BNE bad

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
  CMP r11, r29
  BEQ bad
  LI r1, 7
  LI r2, 9
  CALL_CAP r23, fd8, r1, r2
  LI r24, 16
  CMP r23, r24
  BNE bad
  LI r24, 9
  CMP r30, r24
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 19
  WRITE_FD fd1, r1, r2
  EXIT r0

service:
  ADD r3, r1, r2
  RET_CAP r0, r3, r2

bad:
  LI r1, 1
  EXIT r1

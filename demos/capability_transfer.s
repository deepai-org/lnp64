.data
_epdesc: .zero 32
ok_msg: .string "cap transfer ok\n"
obj: .zero 72
cap: .zero 32
payload: .string "R"
out: .zero 8

.text
  LI r29, -1
  LI r10, obj
  LI r20, cap
  LI r13, 1

create_queue:
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

create_memory_object:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 3
  ST [r10, 8], r1
  ST [r10, 16], r0
  LI r1, 5
  ST [r10, 24], r1
  ST [r10, 32], r0
  LI r1, 16
  ST [r10, 40], r1
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad

send_capability:
  LI r1, 4
  ST [r20, 0], r1
  LI r1, 5
  ST [r20, 8], r1
  ST [r20, 16], r0
  ST [r20, 24], r0
  CAP_SEND r14, r20
  BNE r14, r13, bad

receive_read_only:
  LI r1, 3
  ST [r20, 0], r1
  LI r1, 6
  ST [r20, 8], r1
  LI r1, 5
  ST [r20, 16], r1
  ST [r20, 24], r0
  CAP_RECV r15, r20
  BEQ r15, r29, bad

write_original:
  LI r12, payload
  LI r25, 5
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd5 -> send over byte-fd
  BNE r1, r13, bad

read_received:
  LI r16, out
  LI r25, 6
  LI r24, _epdesc
  ST [r24, 0], r16
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  RECV r26, r25, r24  # read_fd fd6 -> recv over byte-fd
  BNE r1, r13, bad
  LD.B r17, [r16, 0]
  LI r18, 82
  BNE r17, r18, bad

write_denied_after_narrow:
  LI r25, 6
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd6 -> send over byte-fd
  BNE r1, r29, bad
  ERRNO_GET r19
  LI r1, 1
  BNE r19, r1, bad

revoke_source_lineage:
  LI r1, 5
  ST [r20, 0], r1
  CAP_REVOKE r21, r20
  BLE r21, r0, bad
  LI r25, 6
  LI r24, _epdesc
  ST [r24, 0], r16
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  RECV r26, r25, r24  # read_fd fd6 -> recv over byte-fd
  ERRNO_GET r22
  LI r1, 116
  BNE r22, r1, bad

done:
  LI r1, ok_msg
  LI r2, 16
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

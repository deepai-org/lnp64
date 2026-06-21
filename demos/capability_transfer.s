.data
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
  WRITE_FD fd5, r12, r13
  BNE r1, r13, bad

read_received:
  LI r16, out
  READ_FD fd6, r16, r13
  BNE r1, r13, bad
  LD.B r17, [r16, 0]
  LI r18, 82
  BNE r17, r18, bad

write_denied_after_narrow:
  WRITE_FD fd6, r12, r13
  BNE r1, r29, bad
  ERRNO_GET r19
  LI r1, 1
  BNE r19, r1, bad

revoke_source_lineage:
  LI r1, 5
  ST [r20, 0], r1
  CAP_REVOKE r21, r20
  BLE r21, r0, bad
  READ_FD fd6, r16, r13
  ERRNO_GET r22
  LI r1, 116
  BNE r22, r1, bad

done:
  LI r1, ok_msg
  LI r2, 16
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

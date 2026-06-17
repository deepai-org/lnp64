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
  CMP r11, r29
  BEQ bad

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
  CMP r11, r29
  BEQ bad

send_capability:
  LI r1, 4
  ST [r20, 0], r1
  LI r1, 5
  ST [r20, 8], r1
  ST [r20, 16], r0
  ST [r20, 24], r0
  CAP_SEND r14, r20
  CMP r14, r13
  BNE bad

receive_read_only:
  LI r1, 3
  ST [r20, 0], r1
  LI r1, 6
  ST [r20, 8], r1
  LI r1, 5
  ST [r20, 16], r1
  ST [r20, 24], r0
  CAP_RECV r15, r20
  CMP r15, r29
  BEQ bad

write_original:
  LI r12, payload
  WRITE_FD fd5, r12, r13
  CMP r1, r13
  BNE bad

read_received:
  LI r16, out
  READ_FD fd6, r16, r13
  CMP r1, r13
  BNE bad
  LD.B r17, [r16, 0]
  LI r18, 82
  CMP r17, r18
  BNE bad

write_denied_after_narrow:
  WRITE_FD fd6, r12, r13
  CMP r1, r29
  BNE bad
  ERRNO_GET r19
  LI r1, 1
  CMP r19, r1
  BNE bad

revoke_source_lineage:
  LI r1, 5
  ST [r20, 0], r1
  CAP_REVOKE r21, r20
  CMP r21, r0
  BLE bad
  READ_FD fd6, r16, r13
  ERRNO_GET r22
  LI r1, 116
  CMP r22, r1
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 16
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

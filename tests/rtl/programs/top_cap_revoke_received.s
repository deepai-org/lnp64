.data
object_arg: .zero 72
cap_arg: .zero 32

.text
  LI r29, -1
  LI r10, object_arg
  LI r20, cap_arg

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
  BNE r11, r0, bad

send_stdout_cap:
  LI r1, 4
  ST [r20, 0], r1
  LI r1, 1
  ST [r20, 8], r1
  ST [r20, 16], r0
  ST [r20, 24], r0
  CAP_SEND r12, r20
  LI r1, 1
  BNE r12, r1, bad

receive_capability:
  LI r1, 3
  ST [r20, 0], r1
  LI r1, 5
  ST [r20, 8], r1
  ST [r20, 16], r0
  ST [r20, 24], r0
  CAP_RECV r13, r20
  BEQ r13, r29, bad

revoke_source_lineage:
  LI r1, 1
  ST [r20, 0], r1
  ST [r20, 8], r0
  ST [r20, 16], r0
  ST [r20, 24], r0
  CAP_REVOKE r14, r20
  BEQ r14, r29, bad

received_token_is_stale:
  ST [r20, 0], r13
  LI r1, 6
  ST [r20, 8], r1
  ST [r20, 16], r0
  ST [r20, 24], r0
  CAP_DUP r15, r20
  BNE r15, r29, bad
  ERRNO_GET r16
  LI r1, 116
  BNE r16, r1, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

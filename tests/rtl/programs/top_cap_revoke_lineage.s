.data
cap_arg: .zero 32

.text
  LI r29, -1
  LI r10, cap_arg

dup_revoke_rights_to_fd4:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 4
  ST [r10, 8], r1
  LI r1, 129
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r11, r10
  BEQ r11, r29, bad

revoke_source_lineage:
  LI r1, 1
  ST [r10, 0], r1
  CAP_REVOKE r12, r10
  LI r1, 1
  BLE r12, r1, bad

old_child_token_rejected:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  ST [r10, 16], r0
  ST [r10, 24], r0
  CAP_DUP r13, r10
  BNE r13, r29, bad
  ERRNO_GET r14
  LI r1, 116
  BNE r14, r1, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

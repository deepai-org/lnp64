.data
cap_arg: .zero 32

.text
  LI r29, -1
  LI r10, cap_arg

forged_token_rejected:
  LI r11, 1073741824
  LSLI r11, r11, 32
  LI r1, 1
  LSLI r1, r1, 8
  OR r11, r11, r1
  LI r1, 5
  OR r11, r11, r1
  ST [r10, 0], r11
  LI r1, 6
  ST [r10, 8], r1
  ST [r10, 16], r0
  ST [r10, 24], r0
  CAP_DUP r12, r10
  CMP r12, r29
  BNE bad
  ERRNO_GET r13
  LI r1, 116
  CMP r13, r1
  BNE bad

failed_dup_did_not_install_fd6:
  LI r1, 6
  ST [r10, 0], r1
  LI r1, 7
  ST [r10, 8], r1
  ST [r10, 16], r0
  ST [r10, 24], r0
  CAP_DUP r14, r10
  CMP r14, r29
  BNE bad
  ERRNO_GET r15
  LI r1, 9
  CMP r15, r1
  BNE bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

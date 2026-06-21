.data
ok_msg: .string "env get ok\n"
topology: .zero 320

.text
  LI r29, -1

check_isa:
  LI r2, 1
  ENV_GET r1, r2, r0, r0
  BEQ r1, r29, bad
  LI r3, 1
  BNE r1, r3, bad

check_hwcap:
  LI r2, 5
  ENV_GET r1, r2, r0, r0
  LI r3, 128
  AND r4, r1, r3
  BEQ r4, r0, bad

check_classifier_features:
  LI r2, 29
  ENV_GET r1, r2, r0, r0
  LI r3, 8
  AND r4, r1, r3
  BEQ r4, r0, bad

check_security_features:
  LI r2, 27
  ENV_GET r1, r2, r0, r0
  LI r3, 448
  AND r4, r1, r3
  BNE r4, r3, bad

check_topology_scalars:
  LI r2, 30
  ENV_GET r1, r2, r0, r0
  LI r3, 5
  BNE r1, r3, bad
  LI r2, 31
  ENV_GET r1, r2, r0, r0
  LI r3, 1
  BNE r1, r3, bad

check_topology_records:
  LI r2, 65
  LI r5, topology
  LI r6, 320
  ENV_GET r1, r2, r5, r6
  BNE r1, r6, bad
  LD r7, [r5, 0]
  LI r3, 1
  BNE r7, r3, bad
  LD r8, [r5, 192]
  LI r3, 4
  BNE r8, r3, bad
  LD r9, [r5, 232]
  LI r3, 4096
  BNE r9, r3, bad
  LD r10, [r5, 256]
  LI r3, 5
  BNE r10, r3, bad
  LD r11, [r5, 272]
  LI r3, 4096
  BNE r11, r3, bad

done:
  LI r1, ok_msg
  LI r2, 11
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

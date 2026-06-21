.data
ok_msg: .string "resource domains ok\n"
arg: .zero 208

.text
  LI r10, arg
  LI r28, -1

create_vm:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r0
  ST [r10, 16], r0
  LI r1, 1
  ST [r10, 24], r1
  LI r1, 100
  ST [r10, 32], r1
  LI r1, 8192
  ST [r10, 40], r1
  LI r1, 8
  ST [r10, 48], r1
  LI r1, 32
  ST [r10, 56], r1
  LI r1, 255
  ST [r10, 64], r1
  LI r1, 15
  ST [r10, 72], r1
  DOMAIN_CTL r20, r10
  BEQ r20, r28, bad

nested_vm:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  ST [r10, 24], r1
  LI r1, 50
  ST [r10, 32], r1
  LI r1, 4096
  ST [r10, 40], r1
  LI r1, 4
  ST [r10, 48], r1
  LI r1, 16
  ST [r10, 56], r1
  LI r1, 127
  ST [r10, 64], r1
  LI r1, 7
  ST [r10, 72], r1
  DOMAIN_CTL r21, r10
  BEQ r21, r28, bad

create_container:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r0
  ST [r10, 16], r0
  LI r1, 2
  ST [r10, 24], r1
  LI r1, 80
  ST [r10, 32], r1
  LI r1, 4096
  ST [r10, 40], r1
  LI r1, 6
  ST [r10, 48], r1
  LI r1, 24
  ST [r10, 56], r1
  LI r1, 63
  ST [r10, 64], r1
  LI r1, 3
  ST [r10, 72], r1
  DOMAIN_CTL r22, r10
  BEQ r22, r28, bad

nested_container:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r22
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 2
  ST [r10, 24], r1
  LI r1, 40
  ST [r10, 32], r1
  LI r1, 2048
  ST [r10, 40], r1
  LI r1, 3
  ST [r10, 48], r1
  LI r1, 12
  ST [r10, 56], r1
  LI r1, 31
  ST [r10, 64], r1
  LI r1, 1
  ST [r10, 72], r1
  DOMAIN_CTL r23, r10
  BEQ r23, r28, bad

create_sandbox:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r0
  ST [r10, 16], r0
  LI r1, 8
  ST [r10, 24], r1
  LI r1, 60
  ST [r10, 32], r1
  LI r1, 2048
  ST [r10, 40], r1
  LI r1, 3
  ST [r10, 48], r1
  LI r1, 10
  ST [r10, 56], r1
  LI r1, 15
  ST [r10, 64], r1
  LI r1, 1
  ST [r10, 72], r1
  DOMAIN_CTL r24, r10
  BEQ r24, r28, bad

nested_sandbox:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r24
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 8
  ST [r10, 24], r1
  LI r1, 30
  ST [r10, 32], r1
  LI r1, 1024
  ST [r10, 40], r1
  LI r1, 2
  ST [r10, 48], r1
  LI r1, 5
  ST [r10, 56], r1
  LI r1, 7
  ST [r10, 64], r1
  LI r1, 1
  ST [r10, 72], r1
  DOMAIN_CTL r25, r10
  BEQ r25, r28, bad

check_nested_vm:
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r21
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r26, r10
  LI r1, 200
  BNE r26, r1, bad
  LD r26, [r10, 120]
  BNE r26, r20, bad
  LD r26, [r10, 128]
  LI r1, 2
  BNE r26, r1, bad

check_nested_container_freezer:
  LI r1, 4
  ST [r10, 0], r1
  ST [r10, 8], r22
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r26, r10
  BNE r26, r0, bad
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r23
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r26, r10
  LD r26, [r10, 112]
  LI r1, 1
  BNE r26, r1, bad
  LI r1, 5
  ST [r10, 0], r1
  ST [r10, 8], r22
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r26, r10
  BNE r26, r0, bad

check_sandbox_revocation:
  LI r1, 2
  ST [r10, 0], r1
  ST [r10, 8], r24
  LI r1, 1
  ST [r10, 16], r1
  ST [r10, 24], r0
  ST [r10, 32], r0
  ST [r10, 40], r0
  ST [r10, 48], r0
  ST [r10, 56], r0
  LI r1, 3
  ST [r10, 64], r1
  ST [r10, 72], r0
  DOMAIN_CTL r26, r10
  BNE r26, r0, bad
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r25
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r26, r10
  LD r26, [r10, 64]
  LI r1, 3
  BNE r26, r1, bad

check_monotonic_limit_reject:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r24
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 8
  ST [r10, 24], r1
  LI r1, 61
  ST [r10, 32], r1
  LI r1, 2048
  ST [r10, 40], r1
  LI r1, 3
  ST [r10, 48], r1
  LI r1, 10
  ST [r10, 56], r1
  LI r1, 3
  ST [r10, 64], r1
  LI r1, 1
  ST [r10, 72], r1
  DOMAIN_CTL r26, r10
  BNE r26, r28, bad

check_stale_generation_reject:
  LI r1, 6
  ST [r10, 0], r1
  ST [r10, 8], r25
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r26, r10
  BNE r26, r0, bad
  LI r1, 3
  ST [r10, 0], r1
  DOMAIN_CTL r26, r10
  BNE r26, r28, bad

done:
  LI r1, ok_msg
  LI r2, 20
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

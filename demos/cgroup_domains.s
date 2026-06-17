.data
ok_msg: .string "cgroup domains ok\n"
dom: .zero 208
obj: .zero 64

.text
  LI r10, dom
  LI r11, -1

create_limited_domain:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r0
  ST [r10, 16], r0
  LI r1, 4
  ST [r10, 24], r1
  LI r1, 1000
  ST [r10, 32], r1
  LI r1, 5000000
  ST [r10, 40], r1
  LI r1, 1
  ST [r10, 48], r1
  LI r1, 5
  ST [r10, 56], r1
  LI r1, 63
  ST [r10, 64], r1
  ST [r10, 72], r1
  DOMAIN_CTL r20, r10
  CMP r20, r11
  BEQ bad

freeze_resume:
  LI r1, 4
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r21, r10
  CMP r21, r0
  BNE bad
  LI r1, 3
  ST [r10, 0], r1
  DOMAIN_CTL r21, r10
  LD r22, [r10, 112]
  LI r1, 1
  CMP r22, r1
  BNE bad
  LI r1, 5
  ST [r10, 0], r1
  DOMAIN_CTL r21, r10
  CMP r21, r0
  BNE bad

attach_self:
  LI r1, 7
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r21, r10
  CMP r21, r0
  BNE bad

baseline_usage:
  LI r1, 3
  ST [r10, 0], r1
  DOMAIN_CTL r21, r10
  LD r22, [r10, 88]
  LD r23, [r10, 96]
  LD r24, [r10, 104]
  LI r1, 1
  CMP r23, r1
  BNE bad

memory_pressure:
  LI r1, 64
  ALLOC r25, r1
  CMP r25, r11
  BEQ bad
  LI r1, 3
  ST [r10, 0], r1
  DOMAIN_CTL r21, r10
  LD r26, [r10, 88]
  CMP r26, r22
  BLE bad
  FREE r25
  LI r1, 3
  ST [r10, 0], r1
  DOMAIN_CTL r21, r10
  LD r26, [r10, 88]
  CMP r26, r22
  BNE bad
  LI r1, 1000000
  ALLOC r25, r1
  CMP r25, r11
  BNE bad

pid_limit:
  LI r1, unused_worker
  SPAWN r25, r1
  CMP r25, r11
  BNE bad

fdr_limit:
  LI r12, obj
  LI r1, 1
  ST [r12, 0], r1
  LI r1, 2
  ST [r12, 8], r1
  LI r1, 1
  ST [r12, 16], r1
  LI r1, 3
  ST [r12, 24], r1
  LI r1, 4
  ST [r12, 32], r1
  OBJECT_CTL r25, r12
  CMP r25, r0
  BNE bad
  LI r1, 3
  ST [r10, 0], r1
  DOMAIN_CTL r21, r10
  LD r26, [r10, 104]
  CMP r26, r24
  BLE bad
  FD_DUP2 fd5, fd4
  CMP r1, r11
  BNE bad
  FD_CLOSE fd3
  FD_CLOSE fd4
  LI r1, 3
  ST [r10, 0], r1
  DOMAIN_CTL r21, r10
  LD r26, [r10, 104]
  CMP r26, r24
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 18
  WRITE_FD fd1, r1, r2
  EXIT r0

unused_worker:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

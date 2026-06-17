.data
ok_msg: .string "sandboxed domain ok\n"
dom: .zero 208

.text
  LI r10, dom
  LI r29, -1

create_sandbox:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r0
  ST [r10, 16], r0
  LI r1, 8
  ST [r10, 24], r1
  LI r1, 100
  ST [r10, 32], r1
  LI r1, 4096
  ST [r10, 40], r1
  LI r1, 1
  ST [r10, 48], r1
  LI r1, 4
  ST [r10, 56], r1
  LI r1, 2
  ST [r10, 64], r1
  ST [r10, 72], r0
  DOMAIN_CTL r20, r10
  CMP r20, r29
  BEQ bad

query_sandbox:
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r21, r10
  LI r1, 200
  CMP r21, r1
  BNE bad
  LD r22, [r10, 64]
  LI r1, 2
  CMP r22, r1
  BNE bad

reject_broader_child:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 8
  ST [r10, 24], r1
  LI r1, 10
  ST [r10, 32], r1
  LI r1, 1024
  ST [r10, 40], r1
  LI r1, 1
  ST [r10, 48], r1
  ST [r10, 56], r1
  LI r1, 10
  ST [r10, 64], r1
  ST [r10, 72], r0
  DOMAIN_CTL r23, r10
  CMP r23, r29
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 20
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

.data
dom: .zero 208

.text
  LI r10, dom
  LI r11, -1

create_domain:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r0
  ST [r10, 16], r0
  LI r1, 4
  ST [r10, 24], r1
  LI r1, 100000000
  ST [r10, 32], r1
  LI r1, 100000000
  ST [r10, 40], r1
  LI r1, 32
  ST [r10, 48], r1
  LI r1, 64
  ST [r10, 56], r1
  LI r1, 63
  ST [r10, 64], r1
  ST [r10, 72], r1
  DOMAIN_CTL r20, r10
  CMP r20, r11
  BEQ bad

attach_self:
  LI r1, 7
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r21, r10
  CMP r21, r0
  BNE bad

query_baseline:
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r21, r10
  LD r25, [r10, 88]
  LD r24, [r10, 96]
  LI r1, 1
  CMP r24, r1
  BNE bad

charge_memory:
  LI r1, 64
  ALLOC r22, r1
  CMP r22, r11
  BEQ bad

query_charged:
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r21, r10
  LD r23, [r10, 88]
  CMP r23, r25
  BLE bad
  LD r24, [r10, 96]
  LI r1, 1
  CMP r24, r1
  BNE bad

release_memory:
  FREE r22
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r21, r10
  LD r23, [r10, 88]
  CMP r23, r25
  BNE bad
  LI r23, 0
  LI r25, 0
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

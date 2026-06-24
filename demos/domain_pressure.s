.data
_epdesc: .zero 32
ok_msg: .string "domain pressure ok\n"
dom: .zero 208
worker_retval: .quad 0

.text
  LI r10, dom
  LI r11, -1

create_parent:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r0
  ST [r10, 16], r0
  LI r1, 80
  ST [r10, 24], r1
  LI r1, 1000000
  ST [r10, 32], r1
  LI r1, 7000000
  ST [r10, 40], r1
  LI r1, 8
  ST [r10, 48], r1
  LI r1, 32
  ST [r10, 56], r1
  LI r1, 63
  ST [r10, 64], r1
  ST [r10, 72], r1
  DOMAIN_CTL r20, r10
  LI r29, 10
  BEQ r20, r11, bad

create_child:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 4
  ST [r10, 24], r1
  LI r1, 100000
  ST [r10, 32], r1
  LI r1, 6500000
  ST [r10, 40], r1
  LI r1, 4
  ST [r10, 48], r1
  LI r1, 16
  ST [r10, 56], r1
  LI r1, 63
  ST [r10, 64], r1
  ST [r10, 72], r1
  DOMAIN_CTL r21, r10
  LI r29, 20
  BEQ r21, r11, bad

create_grandchild:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r21
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 4
  ST [r10, 24], r1
  LI r1, 10000
  ST [r10, 32], r1
  LI r1, 6000000
  ST [r10, 40], r1
  LI r1, 2
  ST [r10, 48], r1
  LI r1, 8
  ST [r10, 56], r1
  LI r1, 63
  ST [r10, 64], r1
  ST [r10, 72], r1
  DOMAIN_CTL r22, r10
  LI r29, 30
  BEQ r22, r11, bad

baseline_child_counts:
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r20
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r23, r10
  LD r24, [r10, 136]
  LI r1, 1
  LI r29, 40
  BNE r24, r1, bad

  LI r1, pressure_worker
  CLONE.SPAWN r25, r1, r0
  BEQ r25, r11, bad

parent_queries:
  LI r1, 4
  SLEEP r1

  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r22
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r23, r10
  LD r24, [r10, 96]
  LI r1, 1
  LI r29, 50
  BNE r24, r1, bad
  LD r24, [r10, 88]
  LI r29, 51
  BLE r24, r0, bad

  ST [r10, 8], r21
  DOMAIN_CTL r23, r10
  LD r24, [r10, 96]
  LI r1, 1
  LI r29, 52
  BNE r24, r1, bad

  ST [r10, 8], r20
  DOMAIN_CTL r23, r10
  LD r24, [r10, 96]
  LI r1, 1
  LI r29, 53
  BNE r24, r1, bad

  LI r13, worker_retval
join_worker:
  THREAD_JOIN r23, r25, r13
  BEQ r23, r0, joined_worker
  LI r1, 1
  SLEEP r1
  BEQ r0, r0, join_worker
joined_worker:
  LD r24, [r13, 0]
  LI r29, 54
  BNE r24, r0, bad

freeze_resume_subtree:
  LI r1, 4
  ST [r10, 0], r1
  ST [r10, 8], r21
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r23, r10
  LI r29, 60
  BNE r23, r0, bad
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r22
  DOMAIN_CTL r23, r10
  LD r24, [r10, 112]
  LI r1, 1
  LI r29, 61
  BNE r24, r1, bad
  LI r1, 5
  ST [r10, 0], r1
  ST [r10, 8], r21
  DOMAIN_CTL r23, r10
  LI r29, 62
  BNE r23, r0, bad

create_destroy_empty_leaf:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r22
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 4
  ST [r10, 24], r1
  LI r1, 1000
  ST [r10, 32], r1
  LI r1, 100000
  ST [r10, 40], r1
  LI r1, 1
  ST [r10, 48], r1
  LI r1, 4
  ST [r10, 56], r1
  LI r1, 63
  ST [r10, 64], r1
  ST [r10, 72], r1
  DOMAIN_CTL r26, r10
  LI r29, 70
  BEQ r26, r11, bad
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r22
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r23, r10
  LD r24, [r10, 136]
  LI r1, 1
  LI r29, 71
  BNE r24, r1, bad
  LI r1, 6
  ST [r10, 0], r1
  ST [r10, 8], r26
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r23, r10
  LI r29, 72
  BNE r23, r0, bad
  LI r1, 3
  ST [r10, 0], r1
  ST [r10, 8], r22
  DOMAIN_CTL r23, r10
  LD r24, [r10, 136]
  LI r29, 73
  BNE r24, r0, bad

  LI r1, ok_msg
  LI r2, 19
  LI r25, 1
  LI r24, _epdesc
  ST [r24, 0], r1
  ST [r24, 8], r2
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd1 -> send over byte-fd
  EXIT r0

pressure_worker:
  LI r1, 7
  ST [r10, 0], r1
  ST [r10, 8], r22
  LI r1, 1
  ST [r10, 16], r1
  DOMAIN_CTL r23, r10
  LI r29, 80
  BNE r23, r0, bad
  LI r1, 64
  ALLOC r27, r1
  LI r29, 81
  BEQ r27, r11, bad
  LI r28, 80
worker_delay:
  ADDI r28, r28, -1
  BNE r28, r0, worker_delay
  FREE r27
  EXIT r0

bad:
  EXIT r29

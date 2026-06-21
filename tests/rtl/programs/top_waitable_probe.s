.text
  LI r29, -1
  LI r10, 0
  LI r20, 1

create_ready_event_counter:
  LI r1, 1
  ST [r10, 0], r1
  ST [r10, 8], r1
  ST [r10, 16], r1
  LI r1, 4
  ST [r10, 24], r1
  ST [r10, 32], r0
  LI r1, 1
  ST [r10, 40], r1
  ST [r10, 48], r0
  OBJECT_CTL r11, r10
  BEQ r11, r29, bad

probe_static_ready:
  WAITABLE_PROBE r12, fd4, r20
  BNE r12, r20, bad

probe_dynamic_ready:
  LI r16, 4
  WAITABLE_PROBE r17, r16, r20
  BNE r17, r20, bad

drain_and_probe_empty:
  LI r12, 80
  LI r13, 8
  READ_FD fd4, r12, r13
  BNE r1, r13, bad
  WAITABLE_PROBE r18, fd4, r20
  BNE r18, r0, bad

probe_closed_fd_error:
  WAITABLE_PROBE r19, fd7, r20
  LI r1, -9
  BNE r19, r1, bad
  ERRNO_GET r21
  LI r1, 9
  BNE r21, r1, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

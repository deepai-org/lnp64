# Stress catalog: malformed_control_record / object_ctl_bad_profile_no_install.
# Object touched: requested queue-profile FDR slot in OBJECT_CTL create.
# Owner: object/FDR owner engine, not caller-owned descriptor table mutation.
# Authority: current domain object+FDR authority plus requested fd7 install slot.
# Generation: fd7 generation must not advance because no object is installed.
# Trace: OBJECT_CTL reject, ERRNO_GET, failed READ_FD on fd7, EXIT are observable.
# Differential: same source runs under emulator and RTL top-program smoke input.

.data
_epdesc: .zero 32
ok_msg: .string "ok object_ctl_bad_profile_no_install\n"
obj: .zero 72
out: .zero 8

.text
  LI r29, -1
  LI r10, obj

bad_profile_create:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 1
  ST [r10, 8], r1
  LI r1, 99
  ST [r10, 16], r1
  LI r1, 7
  ST [r10, 24], r1
  ST [r10, 32], r0
  ST [r10, 40], r0
  OBJECT_CTL r11, r10
  BNE r11, r29, bad
  ERRNO_GET r12
  LI r13, 22
  BNE r12, r13, bad

fd7_was_not_installed:
  LI r14, out
  LI r15, 1
  LI r25, 7
  LI r24, _epdesc
  ST [r24, 0], r14
  ST [r24, 8], r15
  ST [r24, 16], r0
  ST [r24, 24], r0
  RECV r26, r25, r24  # read_fd fd7 -> recv over byte-fd
  BNE r1, r29, bad
  ERRNO_GET r16
  LI r17, 9
  BNE r16, r17, bad

done:
  LI r1, ok_msg
  LI r2, 37
  LI r25, 1
  LI r24, _epdesc
  ST [r24, 0], r1
  ST [r24, 8], r2
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd1 -> send over byte-fd
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

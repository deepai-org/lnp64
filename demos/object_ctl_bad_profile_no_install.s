# Stress catalog: malformed_control_record / object_ctl_bad_profile_no_install.
# Object touched: requested queue-profile FDR slot in OBJECT_CTL create.
# Owner: object/FDR owner engine, not caller-owned descriptor table mutation.
# Authority: current domain object+FDR authority plus requested fd7 install slot.
# Generation: fd7 generation must not advance because no object is installed.
# Trace: OBJECT_CTL reject, ERRNO_GET, failed READ_FD on fd7, EXIT are observable.
# Differential: same source runs under emulator and RTL top-program smoke input.

.data
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
  CMP r11, r29
  BNE bad
  ERRNO_GET r12
  LI r13, 22
  CMP r12, r13
  BNE bad

fd7_was_not_installed:
  LI r14, out
  LI r15, 1
  READ_FD fd7, r14, r15
  CMP r1, r29
  BNE bad
  ERRNO_GET r16
  LI r17, 9
  CMP r16, r17
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 37
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

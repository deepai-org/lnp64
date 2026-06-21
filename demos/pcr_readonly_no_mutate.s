# Stress catalog: asm_set_pcr_readonly / pcr_readonly_no_mutate.
# Object touched: process and thread PCR metadata.
# Owner: PCR/process metadata owner engine, not caller-side cached register state.
# Authority: writable TP/SIGMASK selectors only; PID/TID/credential/realtime selectors are read-only.
# Generation: process/thread metadata epoch must not advance for rejected read-only writes.
# Trace: GET_PCR, SET_PCR accept, SET_PCR reject, ERRNO_GET, WRITE_FD, EXIT are observable.
# Differential: same source runs under emulator and RTL top-program smoke input.

.data
ok_msg: .string "ok pcr_readonly_no_mutate\n"

.text
  LI r29, -1

check_writable_tp:
  LI r20, 0x1234
  SET_PCR r21, TP, r20
  BNE r21, r0, bad
  GET_PCR r22, TP
  BNE r22, r20, bad

check_writable_sigmask:
  LI r20, 0x55
  SET_PCR r21, SIGMASK, r20
  BNE r21, r0, bad
  GET_PCR r22, SIGMASK
  BNE r22, r20, bad

check_readonly_pid_no_mutate:
  GET_PCR r23, PID
  LI r24, 0x7777
  LI r28, 77
  ERRNO_SET r28
  SET_PCR r25, PID, r24
  BNE r25, r29, bad
  GET_PCR r26, PID
  BNE r26, r23, bad
  ERRNO_GET r27
  BNE r27, r28, bad

check_readonly_tid_no_mutate:
  GET_PCR r23, TID
  SET_PCR r25, TID, r24
  BNE r25, r29, bad
  GET_PCR r26, TID
  BNE r26, r23, bad
  ERRNO_GET r27
  BNE r27, r28, bad

done:
  LI r1, ok_msg
  LI r2, 26
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

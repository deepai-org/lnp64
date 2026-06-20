# Stress catalog: asm_wait_ready_no_consume / waitable_probe_no_consume.
# Object touched: pipe-profile queue created through OBJECT_CTL.
# Owner: object/queue owner engine, not core-private emulator state.
# Authority: fd3 read endpoint and fd4 write endpoint returned by OBJECT_CTL.
# Generation: FDR tokens carry generation; stale use would be rejected by FDR checks.
# Trace: OBJECT_CTL, PUSH/WRITE_FD, WAITABLE_PROBE, PULL/READ_FD are observable.
# Differential: same source runs under emulator and RTL top-program smoke input.

.data
ok_msg: .string "ok waitable_probe_no_consume\n"
obj: .zero 72
payload: .zero 8
out: .zero 8

.text
  LI r29, -1
  LI r10, obj
  LI r20, 1

create_pipe_queue:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 2
  ST [r10, 8], r1
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 3
  ST [r10, 24], r1
  LI r1, 4
  ST [r10, 32], r1
  ST [r10, 40], r0
  OBJECT_CTL r11, r10
  CMP r11, r0
  BNE bad

push_payload:
  LI r12, payload
  LI r13, 8
  LI r1, 42
  ST [r12, 0], r1
  WRITE_FD fd4, r12, r13
  CMP r1, r13
  BNE bad

probe_ready_twice:
  WAITABLE_PROBE r14, fd3, r20
  CMP r14, r20
  BNE bad
  WAITABLE_PROBE r15, fd3, r20
  CMP r15, r20
  BNE bad

pull_after_probes:
  LI r16, out
  READ_FD fd3, r16, r13
  CMP r1, r13
  BNE bad
  LD r17, [r16, 0]
  LI r18, 42
  CMP r17, r18
  BNE bad

probe_empty_after_pull:
  WAITABLE_PROBE r19, fd3, r20
  CMP r19, r0
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 29
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

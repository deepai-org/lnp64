# Stress catalog: asm_wait_ready_no_consume / waitable_probe_no_consume.
# Object touched: pipe-profile queue created through OBJECT_CTL.
# Owner: object/queue owner engine, not core-private emulator state.
# Authority: fd3 read endpoint and fd4 write endpoint returned by OBJECT_CTL.
# Generation: FDR tokens carry generation; stale use would be rejected by FDR checks.
# Trace: OBJECT_CTL, PUSH/WRITE_FD, WAITABLE_PROBE, PULL/READ_FD are observable.
# Differential: same source runs under emulator and RTL top-program smoke input.

.data
_epdesc: .zero 32
ok_msg: .string "ok waitable_probe_no_consume\n"
obj: .zero 72
payload: .zero 8
out: .zero 8
ws: .zero 16
wentry: .zero 24

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
  BNE r11, r0, bad

push_payload:
  LI r12, payload
  LI r13, 8
  LI r1, 42
  ST [r12, 0], r1
  LI r25, 4
  LI r24, _epdesc
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd4 -> send over byte-fd
  BNE r1, r13, bad

build_waitset:
  LI r5, ws
  LI r6, wentry
  LI r8, 0               # timeout = 0 (non-blocking)
  ST [r5, 0], r6         # waitset.entries_ptr
  ST [r5, 8], r20        # waitset.count = 1
  LI r1, 3
  ST [r6, 0], r1         # entry.handle = fd3 reader
  ST [r6, 8], r20        # entry.events = POLLIN

probe_ready_twice:
  ST [r6, 16], r0        # clear revents (probing must not consume)
  WAIT r14, r5, r8
  BNE r14, r20, bad
  ST [r6, 16], r0
  WAIT r15, r5, r8
  BNE r15, r20, bad

pull_after_probes:
  LI r16, out
  LI r25, 3
  LI r24, _epdesc
  ST [r24, 0], r16
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  RECV r26, r25, r24  # read_fd fd3 -> recv over byte-fd
  BNE r1, r13, bad
  LD r17, [r16, 0]
  LI r18, 42
  BNE r17, r18, bad

probe_empty_after_pull:
  ST [r6, 16], r0
  WAIT r19, r5, r8
  BNE r19, r0, bad

done:
  LI r1, ok_msg
  LI r2, 29
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

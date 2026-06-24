.text
# EP-I-full-b: await_ex (0x71) retired — its non-blocking poll modes (0/1) are the
# wait verb with timeout=0; the mode-4 block is wait with WAIT_FOREVER; its
# invalid-mode EINVAL check has no wait analog (wait callers pick a timeout, there
# is no mode) and retires with the opcode. Covers ready (pipe has data) then
# not-ready (pipe drained) over a pipe reader — unambiguous byte-exact readiness.
  LI r29, -1
  LI r10, 0
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

send_one_byte:
  LI r3, 3                 # ep handle for the pipe reader fd 3
  LI r4, 4                 # ep handle for the pipe writer fd 4
  LI r12, 80
  LI r1, 81
  ST.B [r12, 0], r1
  LI r13, 1
  LI r24, 160              # send msg descriptor
  ST [r24, 0], r12
  ST [r24, 8], r13
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r2, r4, r24         # send 1 byte over the pipe writer fd4
  BNE r2, r13, bad

build_waitset:
  LI r5, 128             # entries_ptr
  LI r7, 96              # waitset
  LI r8, 0               # timeout = 0 (non-blocking; the old poll modes 0/1)
  LI r6, 3
  ST [r5, 0], r6         # entry.handle = pipe reader fd 3
  ST [r5, 8], r20        # entry.events = POLLIN
  ST [r7, 0], r5         # waitset.entries_ptr
  ST [r7, 8], r20        # waitset.count = 1

poll_ready:
  ST [r5, 16], r0
  WAIT r14, r7, r8
  BNE r14, r20, bad      # 1 ready (pipe has data)
  LD r9, [r5, 16]
  BNE r9, r20, bad       # revents = POLLIN

drain_pipe:
  LI r16, 88
  LI r13, 1
  LI r25, 192              # recv msg descriptor
  ST [r25, 0], r16
  ST [r25, 8], r13
  ST [r25, 16], r0
  ST [r25, 24], r0
  RECV r2, r3, r25        # drain the byte; pipe reader now empty

poll_not_ready:
  ST [r5, 16], r0
  WAIT r17, r7, r8
  BNE r17, r0, bad       # 0 ready
  LD r9, [r5, 16]
  BNE r9, r0, bad        # revents = 0

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

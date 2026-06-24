.text
# EP-I-full-b: readiness probing via the wait verb (waitable_probe 0x6f retired).
# A single-entry waitset polls one fd; the ready count lands in rd and revents in
# entry[16]. Covers the ready case (POLLIN) and the bad-fd case (POLLNVAL=32,
# which counts toward the ready count per POSIX poll) — the failure the retired
# probe expressed as a negative errno is now POLLNVAL in revents.
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

build_waitset:
  LI r5, 128             # entries_ptr
  LI r7, 96              # waitset
  LI r8, 0               # timeout = 0 (non-blocking)
  ST [r7, 0], r5         # waitset.entries_ptr
  ST [r7, 8], r20        # waitset.count = 1

poll_ready:
  LI r6, 4               # handle = ready event-counter fd 4
  ST [r5, 0], r6
  ST [r5, 8], r20        # events = POLLIN
  ST [r5, 16], r0        # clear revents
  WAIT r2, r7, r8
  BNE r2, r20, bad       # 1 ready
  LD r9, [r5, 16]
  BNE r9, r20, bad       # revents = POLLIN

poll_bad_fd_pollnval:
  LI r6, 7               # handle = closed/unallocated fd 7
  ST [r5, 0], r6
  ST [r5, 8], r20        # events = POLLIN
  ST [r5, 16], r0
  WAIT r2, r7, r8
  BNE r2, r20, bad       # POLLNVAL counts as 1 ready
  LD r9, [r5, 16]
  LI r1, 32              # POLLNVAL
  BNE r9, r1, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

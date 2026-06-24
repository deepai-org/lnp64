.text
# EP-I-full-a: unified wait verb (0x86) over a single-entry waitset, non-blocking
# (timeout=0). Creates a ready event-counter fd (same control block as
# top_waitable_probe), builds a waitset { entries_ptr@0, count@8 } whose one entry
# is { handle@0, events@8(=POLLIN), revents@16 }, then WAIT returns the ready count
# in rd and writes revents back into the entry. The RTL resolves the entry by
# double-indirection and reuses the waitable readiness logic.
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
  LI r6, 4               # entry.handle = event-counter fd 4
  ST [r5, 0], r6
  LI r6, 1               # entry.events = POLLIN
  ST [r5, 8], r6
  ST [r5, 16], r0        # entry.revents = 0 (cleared; wait writes it back)
  LI r7, 96              # waitset
  ST [r7, 0], r5         # waitset.entries_ptr = 128
  LI r6, 1
  ST [r7, 8], r6         # waitset.count = 1

do_wait:
  LI r8, 0               # timeout = 0 (non-blocking poll)
  WAIT r2, r7, r8        # r2 = ready count
  BNE r2, r20, bad       # expect 1 ready
  LD r9, [r5, 16]        # revents written back
  BNE r9, r20, bad       # expect POLLIN (1)
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

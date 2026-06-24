.data
_epdesc: .zero 32
ok_msg: .string "stale fd token ok\n"
path: .string "Cargo.toml"
out: .zero 8
desc: .zero 32

.text
  LI r29, -1

open_first:
  LI r1, path
  LI r2, 0
  OPEN_FD_DYN r3, r1, r2
  BEQ r3, r29, bad
  MOV r20, r3

close_first:
  FD_CLOSE_DYN r3
  ERRNO_GET r4
  BNE r4, r0, bad

reopen_same_slot:
  LI r1, path
  LI r2, 0
  OPEN_FD_DYN r5, r1, r2
  BEQ r5, r29, bad
  BEQ r5, r20, bad

# F1-step-2: recv over the unified verb (READ_FD_DYN/0x3b retired). The msg
# descriptor names the receive buffer; the endpoint handle is the fd token, so a
# stale token is rejected by recv's handle resolution exactly as before.
stale_token_rejected:
  LI r6, out
  LI r7, 4
  LI r9, desc
  ST [r9, 0], r6        # bytes_ptr = out
  ST [r9, 8], r7        # bytes_len = 4
  ST [r9, 16], r0       # caps_ptr = 0
  ST [r9, 24], r0       # caps_len = 0
  RECV r2, r20, r9      # recv via the stale token
  ERRNO_GET r8
  LI r1, 116
  BNE r8, r1, bad

fresh_token_still_reads:
  RECV r2, r5, r9       # recv via the freshly reopened token
  BNE r1, r7, bad

done:
  LI r1, ok_msg
  LI r2, 18
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

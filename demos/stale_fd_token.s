.data
ok_msg: .string "stale fd token ok\n"
path: .string "Cargo.toml"
out: .zero 8

.text
  LI r29, -1

open_first:
  LI r1, path
  LI r2, 0
  OPEN_FD_DYN r3, r1, r2
  CMP r3, r29
  BEQ bad
  MOV r20, r3

close_first:
  FD_CLOSE_DYN r3
  ERRNO_GET r4
  CMP r4, r0
  BNE bad

reopen_same_slot:
  LI r1, path
  LI r2, 0
  OPEN_FD_DYN r5, r1, r2
  CMP r5, r29
  BEQ bad
  CMP r5, r20
  BEQ bad

stale_token_rejected:
  LI r6, out
  LI r7, 4
  READ_FD_DYN r20, r6, r7
  ERRNO_GET r8
  LI r1, 116
  CMP r8, r1
  BNE bad

fresh_token_still_reads:
  READ_FD_DYN r5, r6, r7
  CMP r1, r7
  BNE bad

done:
  LI r1, ok_msg
  LI r2, 18
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

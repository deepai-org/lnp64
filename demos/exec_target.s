.data
msg: .string "exec ok\n"

.text
  LI r1, msg
  LI r2, 8
  WRITE_FD fd1, r1, r2
  EXIT r0

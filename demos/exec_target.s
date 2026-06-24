.data
_epdesc: .zero 32
msg: .string "exec ok\n"

.text
  LI r1, msg
  LI r2, 8
  LI r25, 1
  LI r24, _epdesc
  ST [r24, 0], r1
  ST [r24, 8], r2
  ST [r24, 16], r0
  ST [r24, 24], r0
  SEND r26, r25, r24  # write_fd fd1 -> send over byte-fd
  EXIT r0

.data
exec_path: .string "demos/exec_target.s"
ucode: .string "PORT 9 123\n"
system_msg: .string "system ok\n"
futex_word: .quad 0
sig_flag: .quad 0

.text
  LI r1, handler
  LI r2, 10
  SIGACTION r2, r1
  GET_PCR r3, PID
  KILL r3, r2
  YIELD
  LD r20, sig_flag
  LI r4, 1
  CMP r20, r4
  BNE bad

  LI r5, 16
  LI r25, 3
  MMAP r6, r0, r5, r25, fd0, r0
  LI r7, 99
  ST [r6, 0], r7
  LD r8, [r6, 0]
  CMP r8, r7
  BNE bad

  LI r9, ucode
  LI r10, 11
  LOAD_UCODE r9, r10
  LI r11, 9
  INB r12, r11
  LI r13, 123
  CMP r12, r13
  BNE bad

  LI r14, futex_word
  LI r15, 0
  LI r16, waiter
  CLONE.SPAWN r17, r16, r0
  YIELD
  LI r18, 1
  ST [r14, 0], r18
  FUTEX_WAKE r14, r18
  LI r26, 3
  SLEEP r26
  LD r19, [r14, 0]
  LI r21, 2
  CMP r19, r21
  BNE bad

  LI r1, system_msg
  LI r2, 10
  WRITE_FD fd1, r1, r2

  FORK r22
  CMP r22, r0
  BEQ child
  YIELD
  LI r23, exec_path
  EXEC r23, r0

child:
  EXIT r0

waiter:
  FUTEX_WAIT r14, r15
  LI r18, 2
  ST [r14, 0], r18
  EXIT r0

handler:
  LI r20, 1
  ST sig_flag, r20
  SIGRET

bad:
  LI r1, 1
  EXIT r1

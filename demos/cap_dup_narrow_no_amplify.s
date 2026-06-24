# Stress catalog: asm_cap_dup_narrow / cap_dup_narrow_no_amplify.
# Object touched: FDR capability metadata for duplicated fd tokens.
# Owner: M1 capability/FDR owner engine, not caller-side rights arithmetic.
# Authority: fd1 source capability with duplicate authority.
# Generation: fd4 and fd5 tokens carry generations for accepted duplicates.
# Trace: CAP_DUP accept, CAP_DUP reject, ERRNO_GET, CAP_DUP accept, EXIT are observable.
# Differential: same source runs under emulator and RTL top-program smoke input.

.data
_epdesc: .zero 32
ok_msg: .string "ok cap_dup_narrow_no_amplify\n"
cap_arg: .zero 32

.text
  LI r29, -1
  LI r10, cap_arg

create_read_dup_child:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 4
  ST [r10, 8], r1
  LI r1, 65
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r11, r10
  BEQ r11, r29, bad

reject_broader_rights:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  LI r1, 3
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r12, r10
  BNE r12, r29, bad
  ERRNO_GET r13
  LI r1, 1
  BNE r13, r1, bad

allow_further_narrowing:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  LI r1, 1
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r14, r10
  BEQ r14, r29, bad

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

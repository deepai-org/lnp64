# Stress catalog: asm_cap_dup_narrow / cap_dup_narrow_no_amplify.
# Object touched: FDR capability metadata for duplicated fd tokens.
# Owner: M1 capability/FDR owner engine, not caller-side rights arithmetic.
# Authority: fd1 source capability with duplicate authority.
# Generation: fd4 and fd5 tokens carry generations for accepted duplicates.
# Trace: CAP_DUP accept, CAP_DUP reject, ERRNO_GET, CAP_DUP accept, EXIT are observable.
# Differential: same source runs under emulator and RTL top-program smoke input.

.data
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
  CMP r11, r29
  BEQ bad

reject_broader_rights:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  LI r1, 3
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r12, r10
  CMP r12, r29
  BNE bad
  ERRNO_GET r13
  LI r1, 1
  CMP r13, r1
  BNE bad

allow_further_narrowing:
  ST [r10, 0], r11
  LI r1, 5
  ST [r10, 8], r1
  LI r1, 1
  ST [r10, 16], r1
  ST [r10, 24], r0
  CAP_DUP r14, r10
  CMP r14, r29
  BEQ bad

done:
  LI r1, ok_msg
  LI r2, 29
  WRITE_FD fd1, r1, r2
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

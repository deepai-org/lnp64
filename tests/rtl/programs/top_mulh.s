.text
  ; ISA v2 port: the v1 CMP/CSEL.EQ counting idiom is replaced by the RISC-V
  ; SEQ idiom (sub; sltiu rd, tmp, 1) which yields 1 iff the two values are
  ; equal, then the matches are summed -- identical observable result.
  LI r1, 1
  LSLI r1, r1, 32
  MOV r2, r1
  MULH r3, r1, r2
  MULHU r4, r1, r2
  LI r5, -1
  LSLI r5, r5, 32
  MULH r6, r5, r1
  MULHSU r7, r5, r1
  MULHU r8, r5, r1
  LI r9, 1
  LI r10, -1
  LI r11, 0xffffffff
  LI r12, 0
  SUB r20, r3, r9
  SLTIU r13, r20, 1
  SUB r20, r4, r9
  SLTIU r14, r20, 1
  ADD r13, r13, r14
  SUB r20, r6, r10
  SLTIU r15, r20, 1
  ADD r13, r13, r15
  SUB r20, r7, r10
  SLTIU r16, r20, 1
  ADD r13, r13, r16
  SUB r20, r8, r11
  SLTIU r17, r20, 1
  ADD r13, r13, r17
  EXIT r13

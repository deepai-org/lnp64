.data
ucode: .string "PORT 9 123\n"

.text
  LI r1, ucode
  LI r2, 11
  LOAD_UCODE r1, r2
  LI r3, 9
  INB r4, r3
  LI r5, 123
  BNE r4, r5, bad

  LI r6, 7
  LI r7, 42
  OUTB r6, r7
  INB r8, r6
  LI r9, 42
  BNE r8, r9, bad

  EXIT r0

bad:
  LI r1, 1
  EXIT r1

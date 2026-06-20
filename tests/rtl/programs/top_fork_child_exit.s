.text
  FORK r2
  CMP r2, r0
  BEQ child
  YIELD
  LI r3, 2
  EXIT r0

child:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

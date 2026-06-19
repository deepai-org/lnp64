.text
.globl _start
_start:
  li r1, 6
  li r2, 7
  mul r3, r1, r2
  addi r3, r3, -42

  li r4, -1
  zext.b r5, r4
  addi r5, r5, -255
  or r3, r3, r5

  li r6, 1
  clz r7, r6
  addi r7, r7, -63
  or r3, r3, r7

  li r8, 15
  popcnt r9, r8
  addi r9, r9, -4
  or r3, r3, r9

  exit r3

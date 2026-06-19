  CALL setup
  EXIT r1
setup:
  LR_GET r2
  LR_SET r2
  LI r5, 0x1020
  CALL_REG r5
  RET
  LI r1, 99
leaf:
  LR_GET r2
  ADDI r1, r2, -4113
  RET

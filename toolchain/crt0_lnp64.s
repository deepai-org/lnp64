# LNP64 crt0 startup stub v0.
# Contract source for the future LLVM/lld crt0 object; current toy compiler
# startup remains separate smoke infrastructure.

.text
_start:
  LI r7, 0x700000
  LD r1, [r7, 0]
  LI r2, 0x700008
  LI r8, 8
  MUL r3, r1, r8
  ADD r3, r3, r2
  ADD r3, r3, r8
  ERRNO_SET r0
  CALL main
  EXIT r1

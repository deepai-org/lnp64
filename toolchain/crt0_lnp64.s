# LNP64 crt0 startup stub v0.
# Contract source for the future LLVM/lld crt0 object; current toy compiler
# startup remains separate smoke infrastructure.

.text
.globl _start
.type _start,@function
_start:
  LI r7, 0x7000
  LI r8, 0x100
  MUL r7, r7, r8
  LD r1, 0(r7)
  LI r2, 8
  ADD r2, r7, r2
  LI r8, 8
  MUL r3, r1, r8
  ADD r3, r3, r2
  ADD r3, r3, r8
  ERRNO_SET r0
  CALL main
  EXIT r1

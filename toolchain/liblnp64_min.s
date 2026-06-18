# LNP64 minimal libc smoke stubs v0.
# These stubs exist only to prove real Clang objects can be statically linked
# by lld before the native libc/runtime implementation is ready.

.text
.globl write
.type write,@function
write:
  PUSH r1, r1, r2, r3
  RET

.globl alloc
.type alloc,@function
alloc:
  MOV r2, r1
  LA r4, __lnp64_min_heap_cursor
  LD r3, 0(r4)
  LA r1, __lnp64_min_heap
  ADD r1, r1, r3
  ADD r3, r3, r2
  ST r3, 0(r4)
  RET

.globl malloc
.type malloc,@function
malloc:
  CALL alloc
  RET

.globl free
.type free,@function
free:
  LI r1, 0
  RET

.globl _exit
.type _exit,@function
_exit:
  EXIT r1
  RET

.globl exit
.type exit,@function
exit:
  EXIT r1
  RET

.bss
.globl __lnp64_min_heap_cursor
__lnp64_min_heap_cursor:
  .quad 0

.globl __lnp64_min_heap
__lnp64_min_heap:
  .zero 4096

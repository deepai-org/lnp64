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
  LA r1, __lnp64_min_heap
  RET

.globl free
.type free,@function
free:
  LI r1, 0
  RET

.bss
.globl __lnp64_min_heap
__lnp64_min_heap:
  .zero 256

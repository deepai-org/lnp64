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
  ALLOC r1, r1
  RET

.globl malloc
.type malloc,@function
malloc:
  CALL alloc
  RET

.globl calloc
.type calloc,@function
calloc:
  MUL r8, r1, r2
  MOV r1, r8
  CALL malloc
  MOV r2, r0
  MOV r3, r8
  CALL memset
  RET

.globl realloc
.type realloc,@function
realloc:
  CMP r1, r0
  BEQ realloc_malloc
  CMP r2, r0
  BEQ realloc_free
  LA r4, __lnp64_min_realloc_old
  ST r1, 0(r4)
  LA r4, __lnp64_min_realloc_size
  ST r2, 0(r4)
  MOV r1, r2
  CALL malloc
  CMP r1, r0
  BEQ realloc_done
  LA r4, __lnp64_min_realloc_new
  ST r1, 0(r4)
  LA r4, __lnp64_min_realloc_old
  LD r2, 0(r4)
  ALLOC_SIZE r3, r2
  LA r4, __lnp64_min_realloc_size
  LD r5, 0(r4)
  CMPU r3, r5
  BLE realloc_copy
  MOV r3, r5
realloc_copy:
  LA r4, __lnp64_min_realloc_new
  LD r1, 0(r4)
  CALL memcpy
  LA r4, __lnp64_min_realloc_old
  LD r1, 0(r4)
  CALL free
  LA r4, __lnp64_min_realloc_new
  LD r1, 0(r4)
  RET
realloc_malloc:
  MOV r1, r2
  CALL malloc
  RET
realloc_free:
  CALL free
  RET
realloc_done:
  RET

.globl free
.type free,@function
free:
  FREE r1
  LI r1, 0
  RET

.globl strlen
.type strlen,@function
strlen:
  MOV r2, r1
  LI r1, 0
  LI r4, 1
strlen_loop:
  LD.B r3, 0(r2)
  CMP r3, r0
  BEQ strlen_done
  ADD r1, r1, r4
  ADD r2, r2, r4
  JMP strlen_loop
strlen_done:
  RET

.globl memcpy
.type memcpy,@function
memcpy:
  MOV r4, r1
  LI r5, 0
  LI r6, 1
memcpy_loop:
  CMPU r5, r3
  BGE memcpy_done
  LD.B r7, 0(r2)
  ST.B r7, 0(r1)
  ADD r1, r1, r6
  ADD r2, r2, r6
  ADD r5, r5, r6
  JMP memcpy_loop
memcpy_done:
  MOV r1, r4
  RET

.globl memset
.type memset,@function
memset:
  MOV r4, r1
  LI r5, 0
  LI r6, 1
memset_loop:
  CMPU r5, r3
  BGE memset_done
  ST.B r2, 0(r1)
  ADD r1, r1, r6
  ADD r5, r5, r6
  JMP memset_loop
memset_done:
  MOV r1, r4
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
.globl __lnp64_min_realloc_old
__lnp64_min_realloc_old:
  .quad 0

.globl __lnp64_min_realloc_size
__lnp64_min_realloc_size:
  .quad 0

.globl __lnp64_min_realloc_new
__lnp64_min_realloc_new:
  .quad 0

# LNP64 minimal libc smoke stubs v2.
# These stubs exist only to prove real Clang objects can be statically linked
# by lld before the native libc/runtime implementation is ready. v2 ISA.

.text
.globl write
.type write,@function
write:
  push r1, r1, r2, r3
  ret

.globl read
.type read,@function
read:
  pull r1, r1, r2, r3
  ret

.globl alloc
.type alloc,@function
alloc:
  alloc r1, r1
  ret

.globl malloc
.type malloc,@function
malloc:
  jal r1, alloc
  ret

.globl calloc
.type calloc,@function
calloc:
  mul r8, r1, r2
  mov r1, r8
  jal r1, malloc
  mov r2, r0
  mov r3, r8
  jal r1, memset
  ret

.globl realloc
.type realloc,@function
realloc:
  beq r1, r0, realloc_malloc
  beq r2, r0, realloc_free
  auipc r4, __lnp64_min_realloc_old
  sd r1, 0(r4)
  auipc r4, __lnp64_min_realloc_size
  sd r2, 0(r4)
  mov r1, r2
  jal r1, malloc
  beq r1, r0, realloc_done
  auipc r4, __lnp64_min_realloc_new
  sd r1, 0(r4)
  auipc r4, __lnp64_min_realloc_old
  ld r2, 0(r4)
  alloc_size r3, r2
  auipc r4, __lnp64_min_realloc_size
  ld r5, 0(r4)
  bgeu r5, r3, realloc_copy
  mov r3, r5
realloc_copy:
  auipc r4, __lnp64_min_realloc_new
  ld r1, 0(r4)
  jal r1, memcpy
  auipc r4, __lnp64_min_realloc_old
  ld r1, 0(r4)
  jal r1, free
  auipc r4, __lnp64_min_realloc_new
  ld r1, 0(r4)
  ret
realloc_malloc:
  mov r1, r2
  jal r1, malloc
  ret
realloc_free:
  jal r1, free
  ret
realloc_done:
  ret

.globl free
.type free,@function
free:
  free r1
  li r1, 0
  ret

.globl strlen
.type strlen,@function
strlen:
  mov r2, r1
  li r1, 0
  li r4, 1
strlen_loop:
  lbu r3, 0(r2)
  beq r3, r0, strlen_done
  add r1, r1, r4
  add r2, r2, r4
  jmp strlen_loop
strlen_done:
  ret

.globl memcpy
.type memcpy,@function
memcpy:
  mov r4, r1
  li r5, 0
  li r6, 1
memcpy_loop:
  bgeu r5, r3, memcpy_done
  lbu r7, 0(r2)
  sb r7, 0(r1)
  add r1, r1, r6
  add r2, r2, r6
  add r5, r5, r6
  jmp memcpy_loop
memcpy_done:
  mov r1, r4
  ret

.globl memmove
.type memmove,@function
memmove:
  mov r4, r1
  bgeu r2, r1, memmove_forward
  add r1, r1, r3
  add r2, r2, r3
  li r5, 0
  li r6, 1
memmove_backward_loop:
  bgeu r5, r3, memmove_done
  sub r1, r1, r6
  sub r2, r2, r6
  lbu r7, 0(r2)
  sb r7, 0(r1)
  add r5, r5, r6
  jmp memmove_backward_loop
memmove_forward:
  jal r1, memcpy
  ret
memmove_done:
  mov r1, r4
  ret

.globl memcmp
.type memcmp,@function
memcmp:
  li r4, 0
  li r5, 1
memcmp_loop:
  bgeu r4, r3, memcmp_equal
  lbu r6, 0(r1)
  lbu r7, 0(r2)
  bne r6, r7, memcmp_diff
  add r1, r1, r5
  add r2, r2, r5
  add r4, r4, r5
  jmp memcmp_loop
memcmp_diff:
  sub r1, r6, r7
  ret
memcmp_equal:
  li r1, 0
  ret

.globl memset
.type memset,@function
memset:
  mov r4, r1
  li r5, 0
  li r6, 1
memset_loop:
  bgeu r5, r3, memset_done
  sb r2, 0(r1)
  add r1, r1, r6
  add r5, r5, r6
  jmp memset_loop
memset_done:
  mov r1, r4
  ret

.globl _exit
.type _exit,@function
_exit:
  exit r1
  ret

.globl exit
.type exit,@function
exit:
  exit r1
  ret

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

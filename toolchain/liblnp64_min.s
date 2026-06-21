# LNP64 minimal libc smoke stubs v2.
# These stubs exist only to prove real Clang objects can be statically linked
# by lld before the native libc/runtime implementation is ready. v2 ISA ABI:
#   r0=zero  r1=ra  a0..a7=r2..r9  ret=r2  s0..s9=r18..r27  sp=r31
# Non-leaf functions (those that make a nested jal) save/restore r1(ra) on a
# stack frame; values that must survive a nested call live in callee-saved
# s-registers (r18..r27), themselves saved/restored on the frame.

.text
.globl write
.type write,@function
write:
  push r2, r2, r3, r4
  ret

.globl read
.type read,@function
read:
  pull r2, r2, r3, r4
  ret

.globl alloc
.type alloc,@function
alloc:
  alloc r2, r2
  ret

# non-leaf: calls alloc -> save/restore ra
.globl malloc
.type malloc,@function
malloc:
  addi r31, r31, -16
  sd r1, 0(r31)
  jal r1, alloc
  ld r1, 0(r31)
  addi r31, r31, 16
  ret

# non-leaf: count*size must survive the malloc call -> hold in s0=r18.
.globl calloc
.type calloc,@function
calloc:
  addi r31, r31, -16
  sd r1, 0(r31)
  sd r18, 8(r31)
  mul r18, r2, r3
  mov r2, r18
  jal r1, malloc
  mov r3, r0
  mov r4, r18
  jal r1, memset
  ld r18, 8(r31)
  ld r1, 0(r31)
  addi r31, r31, 16
  ret

# non-leaf: calls malloc/memcpy/free. Old ptr / size held across calls.
.globl realloc
.type realloc,@function
realloc:
  beq r2, r0, realloc_malloc
  beq r3, r0, realloc_free
  auipc r4, __lnp64_min_realloc_old
  sd r2, 0(r4)
  auipc r4, __lnp64_min_realloc_size
  sd r3, 0(r4)
  addi r31, r31, -16
  sd r1, 0(r31)
  mov r2, r3
  jal r1, malloc
  beq r2, r0, realloc_done
  auipc r4, __lnp64_min_realloc_new
  sd r2, 0(r4)
  auipc r4, __lnp64_min_realloc_old
  ld r3, 0(r4)
  alloc_size r4, r3
  auipc r5, __lnp64_min_realloc_size
  ld r6, 0(r5)
  bgeu r6, r4, realloc_copy
  mov r4, r6
realloc_copy:
  auipc r5, __lnp64_min_realloc_new
  ld r2, 0(r5)
  auipc r5, __lnp64_min_realloc_old
  ld r3, 0(r5)
  jal r1, memcpy
  auipc r5, __lnp64_min_realloc_old
  ld r2, 0(r5)
  jal r1, free
  auipc r5, __lnp64_min_realloc_new
  ld r2, 0(r5)
realloc_done:
  ld r1, 0(r31)
  addi r31, r31, 16
  ret
realloc_malloc:
  addi r31, r31, -16
  sd r1, 0(r31)
  mov r2, r3
  jal r1, malloc
  ld r1, 0(r31)
  addi r31, r31, 16
  ret
realloc_free:
  addi r31, r31, -16
  sd r1, 0(r31)
  jal r1, free
  ld r1, 0(r31)
  addi r31, r31, 16
  ret

.globl free
.type free,@function
free:
  free r2
  li r2, 0
  ret

.globl strlen
.type strlen,@function
strlen:
  mov r3, r2
  li r2, 0
  li r5, 1
strlen_loop:
  lbu r4, 0(r3)
  beq r4, r0, strlen_done
  add r2, r2, r5
  add r3, r3, r5
  jmp strlen_loop
strlen_done:
  ret

.globl strcmp
.type strcmp,@function
strcmp:
  li r6, 1
strcmp_loop:
  lbu r4, 0(r2)
  lbu r5, 0(r3)
  bne r4, r5, strcmp_diff
  beq r4, r0, strcmp_equal
  add r2, r2, r6
  add r3, r3, r6
  jmp strcmp_loop
strcmp_diff:
  sub r2, r4, r5
  ret
strcmp_equal:
  li r2, 0
  ret

.globl memcpy
.type memcpy,@function
memcpy:
  mov r5, r2
  li r6, 0
  li r7, 1
memcpy_loop:
  bgeu r6, r4, memcpy_done
  lbu r8, 0(r3)
  sb r8, 0(r2)
  add r2, r2, r7
  add r3, r3, r7
  add r6, r6, r7
  jmp memcpy_loop
memcpy_done:
  mov r2, r5
  ret

# non-leaf: forward case calls memcpy.
.globl memmove
.type memmove,@function
memmove:
  mov r5, r2
  bgeu r3, r2, memmove_forward
  add r2, r2, r4
  add r3, r3, r4
  li r6, 0
  li r7, 1
memmove_backward_loop:
  bgeu r6, r4, memmove_done
  sub r2, r2, r7
  sub r3, r3, r7
  lbu r8, 0(r3)
  sb r8, 0(r2)
  add r6, r6, r7
  jmp memmove_backward_loop
memmove_forward:
  addi r31, r31, -16
  sd r1, 0(r31)
  jal r1, memcpy
  ld r1, 0(r31)
  addi r31, r31, 16
  ret
memmove_done:
  mov r2, r5
  ret

.globl memcmp
.type memcmp,@function
memcmp:
  li r5, 0
  li r6, 1
memcmp_loop:
  bgeu r5, r4, memcmp_equal
  lbu r7, 0(r2)
  lbu r8, 0(r3)
  bne r7, r8, memcmp_diff
  add r2, r2, r6
  add r3, r3, r6
  add r5, r5, r6
  jmp memcmp_loop
memcmp_diff:
  sub r2, r7, r8
  ret
memcmp_equal:
  li r2, 0
  ret

.globl memset
.type memset,@function
memset:
  mov r5, r2
  li r6, 0
  li r7, 1
memset_loop:
  bgeu r6, r4, memset_done
  sb r3, 0(r2)
  add r2, r2, r7
  add r6, r6, r7
  jmp memset_loop
memset_done:
  mov r2, r5
  ret

.globl _exit
.type _exit,@function
_exit:
  exit r2
  ret

.globl exit
.type exit,@function
exit:
  exit r2
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

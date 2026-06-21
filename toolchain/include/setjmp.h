#ifndef LNP64_SETJMP_H
#define LNP64_SETJMP_H

#define LNP64_JMPBUF_THREAD_COOKIE 0
#define LNP64_JMPBUF_PROCESS_COOKIE 1
#define LNP64_JMPBUF_STACK_COOKIE 2
#define LNP64_JMPBUF_STACK_POINTER 3
#define LNP64_JMPBUF_LINK_REGISTER 4
/* Callee-saved set s0..s9 = r18..r27 occupy words 5..14. */
#define LNP64_JMPBUF_CALLEE_SAVED_BASE 5
#define LNP64_JMPBUF_CALLEE_SAVED_COUNT 10
#define LNP64_JMPBUF_WORDS 15

typedef unsigned long jmp_buf[LNP64_JMPBUF_WORDS];

int setjmp(jmp_buf env) __attribute__((returns_twice));
void longjmp(jmp_buf env, int value) __attribute__((noreturn));

#define _setjmp  setjmp
#define _longjmp longjmp

#endif

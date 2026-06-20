#ifndef LNP64_SETJMP_H
#define LNP64_SETJMP_H

#define LNP64_JMPBUF_THREAD_COOKIE 0
#define LNP64_JMPBUF_PROCESS_COOKIE 1
#define LNP64_JMPBUF_STACK_COOKIE 2
#define LNP64_JMPBUF_STACK_POINTER 3
#define LNP64_JMPBUF_LINK_REGISTER 4
#define LNP64_JMPBUF_WORDS 5

typedef unsigned long jmp_buf[LNP64_JMPBUF_WORDS];

int setjmp(jmp_buf env) __attribute__((returns_twice));
void longjmp(jmp_buf env, int value) __attribute__((noreturn));

#endif

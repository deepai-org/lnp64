#ifndef LNP64_LIBC_TEST_H
#define LNP64_LIBC_TEST_H

#include <stdint.h>
#include <unistd.h>

extern int t_status;

int t_error(const char *s, ...);
int t_printf(const char *s, ...);
void t_randseed(uint64_t s);
uint64_t t_randn(uint64_t n);
void t_shuffle(uint64_t *p, size_t n);

#endif

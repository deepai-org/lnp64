#ifndef LNP64_LIBC_TEST_H
#define LNP64_LIBC_TEST_H

#include <stdint.h>
#include <unistd.h>

extern int t_status;

int t_error(const char *s, ...);
int t_printf(const char *s, ...);

#endif

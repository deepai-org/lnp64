#include <stdarg.h>
#include <stdio.h>
#include <unistd.h>

#include "test.h"

int t_status = 0;

int t_error(const char *s, ...) {
    t_status = 1;
    write(1, "libc-test failure\n", 18);
    return 0;
}

int t_printf(const char *s, ...) {
    return 0;
}

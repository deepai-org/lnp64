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

static uint64_t t_rand_state = 1;

void t_randseed(uint64_t s) {
    t_rand_state = s ? s : 1;
}

uint64_t t_randn(uint64_t n) {
    t_rand_state ^= t_rand_state << 13;
    t_rand_state ^= t_rand_state >> 7;
    t_rand_state ^= t_rand_state << 17;
    if (n == 0) return 0;
    return t_rand_state % n;
}

void t_shuffle(uint64_t *p, size_t n) {
    size_t i;
    for (i = n; i > 1; i--) {
        size_t j = t_randn(i);
        uint64_t tmp = p[i - 1];
        p[i - 1] = p[j];
        p[j] = tmp;
    }
}

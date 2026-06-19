#ifndef LNP64_FUTEX_H
#define LNP64_FUTEX_H

typedef unsigned long lnp64_word_t;

int futex_wait(volatile lnp64_word_t *addr, lnp64_word_t expected);
int futex_wake(volatile lnp64_word_t *addr, lnp64_word_t count);

#endif

#include <lnp64/futex.h>

#include "lnp64_intrinsics.h"

int futex_wait(volatile lnp64_word_t *addr, lnp64_word_t expected) {
  return (int)__lnp_futex_wait(addr, expected);
}

int futex_wake(volatile lnp64_word_t *addr, lnp64_word_t count) {
  return (int)__lnp_futex_wake(addr, count);
}

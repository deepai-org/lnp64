#include "lnp64_intrinsics.h"

static volatile lnp64_word_t linked_cell = 5;

void _start(void) {
  linked_cell += 2;
  __lnp_exit(linked_cell);
}

#include "lnp64_intrinsics.h"

typedef unsigned long size_t;

void *malloc(size_t size);
void free(void *ptr);
long write(long fd, const void *buf, size_t len);
int futex_wait(volatile lnp64_word_t *addr, lnp64_word_t expected);
int futex_wake(volatile lnp64_word_t *addr, lnp64_word_t count);

static int consumer(lnp64_word_t arg) {
  volatile lnp64_word_t *slot = (volatile lnp64_word_t *)arg;

  while (__sync_val_compare_and_swap(slot, 1, 2) != 1) {
    futex_wait(slot, 0);
  }

  futex_wake(slot, 1);
  __lnp_exit(0);
  return 0;
}

int main(void) {
  volatile lnp64_word_t *slot = (volatile lnp64_word_t *)malloc(sizeof(*slot));
  if (!slot)
    return 1;

  __atomic_store_n(slot, 0, __ATOMIC_SEQ_CST);
  lnp64_word_t tid = __lnp_spawn_entry((lnp64_word_t)consumer,
                                       (lnp64_word_t)slot);
  if (tid == (lnp64_word_t)-1) {
    free((void *)slot);
    return 2;
  }

  __atomic_store_n(slot, 1, __ATOMIC_SEQ_CST);
  futex_wake(slot, 1);

  if (__lnp_thread_join(tid, 0) != 0) {
    free((void *)slot);
    return 3;
  }

  if (__atomic_load_n(slot, __ATOMIC_SEQ_CST) == 2) {
    write(1, "producer consumer ok\n", 21);
    free((void *)slot);
    return 0;
  }

  write(2, "producer consumer failed\n", 25);
  free((void *)slot);
  return 4;
}

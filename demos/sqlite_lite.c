#include <lnp64/intrinsics.h>

typedef unsigned long size_t;

long write(long fd, const void *buf, size_t len);

static volatile lnp64_word_t *db_ptr;

static inline void seq_cst_fence(void) {
  __asm__ volatile("fence.sc" : : : "memory");
}

static int writer(lnp64_word_t arg) {
  (void)arg;
  __sync_val_compare_and_swap((lnp64_word_t *)db_ptr, 0, 1);
  seq_cst_fence();
  __lnp_exit(0);
  return 0;
}

int main(void) {
  db_ptr = (volatile lnp64_word_t *)__lnp_mmap_bootstrap(0, 4096, 3);
  if ((lnp64_word_t)db_ptr == (lnp64_word_t)-1 || db_ptr == 0)
    return 1;

  __atomic_store_n(db_ptr, 0, __ATOMIC_SEQ_CST);
  lnp64_word_t tid = __lnp_spawn_entry((lnp64_word_t)writer, 0);
  if (tid == (lnp64_word_t)-1)
    return 2;
  if (__lnp_thread_join(tid, 0) != 0)
    return 3;

  lnp64_word_t old =
      __sync_val_compare_and_swap((lnp64_word_t *)db_ptr, 1, 2);
  seq_cst_fence();
  if (old == 1) {
    write(1, "sqlite lite ok\n", 15);
    return 0;
  }

  write(2, "sqlite lite failed\n", 19);
  return 4;
}

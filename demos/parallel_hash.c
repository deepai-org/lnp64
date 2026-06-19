#include "lnp64_intrinsics.h"

typedef unsigned long size_t;

void *malloc(size_t size);
void free(void *ptr);
long open(const char *path, int flags);
long read(long fd, void *buf, size_t len);
long write(long fd, const void *buf, size_t len);

static long input_fd;
static volatile lnp64_word_t total;

static int worker(lnp64_word_t arg) {
  (void)arg;
  void *buf = malloc(64);
  if (!buf)
    __lnp_exit(1);

  long n = read(input_fd, buf, 64);
  if (n > 0)
    __atomic_fetch_add(&total, (lnp64_word_t)n, __ATOMIC_SEQ_CST);

  free(buf);
  __lnp_exit(0);
  return 0;
}

int main(void) {
  input_fd = open("demos/hash_input.txt", 0);
  if (input_fd == -1)
    return 1;

  __atomic_store_n(&total, 0, __ATOMIC_SEQ_CST);
  lnp64_word_t t0 = __lnp_spawn_entry((lnp64_word_t)worker, 0);
  lnp64_word_t t1 = __lnp_spawn_entry((lnp64_word_t)worker, 0);
  lnp64_word_t t2 = __lnp_spawn_entry((lnp64_word_t)worker, 0);
  lnp64_word_t t3 = __lnp_spawn_entry((lnp64_word_t)worker, 0);
  if (t0 == (lnp64_word_t)-1 || t1 == (lnp64_word_t)-1 ||
      t2 == (lnp64_word_t)-1 || t3 == (lnp64_word_t)-1)
    return 2;

  if (__lnp_thread_join(t0, 0) != 0 || __lnp_thread_join(t1, 0) != 0 ||
      __lnp_thread_join(t2, 0) != 0 || __lnp_thread_join(t3, 0) != 0)
    return 3;

  if (__atomic_load_n(&total, __ATOMIC_SEQ_CST) == 256) {
    write(1, "parallel hash ok\n", 17);
    return 0;
  }

  write(2, "parallel hash failed\n", 21);
  return 4;
}

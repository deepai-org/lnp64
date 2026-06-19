#include "lnp64_intrinsics.h"

typedef unsigned long size_t;

enum {
  LNP64_EINVAL = 22,
  LNP64_PROT_MASK = 0x7,
};

int lnp64_errno_store(int value);

static void *lnp64_map_failed(int err) {
  lnp64_errno_store(err);
  return (void *)~(lnp64_word_t)0;
}

void *mmap(void *addr, size_t len, int prot, int flags, int fd, long offset) {
  (void)flags;
  if (len == 0 || (prot & ~LNP64_PROT_MASK) != 0)
    return lnp64_map_failed(LNP64_EINVAL);
  if (fd != -1 || offset != 0)
    return lnp64_map_failed(LNP64_EINVAL);
  return __lnp_mmap_bootstrap((lnp64_word_t)addr, (lnp64_word_t)len,
                              (lnp64_word_t)prot);
}

int mprotect(void *addr, size_t len, int prot) {
  if (len == 0 || (prot & ~LNP64_PROT_MASK) != 0) {
    lnp64_errno_store(LNP64_EINVAL);
    return -1;
  }
  return (int)__lnp_mprotect_bootstrap(addr, (lnp64_word_t)len,
                                       (lnp64_word_t)prot);
}

int munmap(void *addr, size_t len) {
  if (len == 0) {
    lnp64_errno_store(LNP64_EINVAL);
    return -1;
  }
  return (int)__lnp_munmap_bootstrap(addr);
}

int msync(void *addr, size_t len, int flags) {
  (void)addr;
  (void)len;
  (void)flags;
  lnp64_errno_store(LNP64_EINVAL);
  return -1;
}

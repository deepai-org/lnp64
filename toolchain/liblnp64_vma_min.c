#include "lnp64_intrinsics.h"

#include <sys/mman.h>

enum {
  LNP64_EINVAL = 22,
  LNP64_PROT_MASK = 0x7,
};

int lnp64_errno_store(int value);

static int lnp64_errno_load(void) {
  unsigned long value;
  __asm__ volatile("errno_get %0" : "=r"(value) : : "memory");
  return (int)value;
}

static int lnp64_complete_status(long status) {
  if (status < 0) {
    lnp64_errno_store(lnp64_errno_load());
    return -1;
  }
  return (int)status;
}

static void *lnp64_complete_ptr(void *value) {
  if (value == (void *)~(lnp64_word_t)0)
    lnp64_errno_store(lnp64_errno_load());
  return value;
}

static void *lnp64_map_failed(int err) {
  lnp64_errno_store(err);
  return (void *)~(lnp64_word_t)0;
}

void *mmap(void *addr, size_t len, int prot, int flags, int fd, off_t offset) {
  (void)flags;
  if (len == 0 || (prot & ~LNP64_PROT_MASK) != 0)
    return lnp64_map_failed(LNP64_EINVAL);
  if (fd != -1 || offset != 0)
    return lnp64_map_failed(LNP64_EINVAL);
  return lnp64_complete_ptr(__lnp_mmap_bootstrap(
      (lnp64_word_t)addr, (lnp64_word_t)len, (lnp64_word_t)prot));
}

int mprotect(void *addr, size_t len, int prot) {
  if (len == 0 || (prot & ~LNP64_PROT_MASK) != 0) {
    lnp64_errno_store(LNP64_EINVAL);
    return -1;
  }
  return lnp64_complete_status((long)__lnp_mprotect_bootstrap(
      addr, (lnp64_word_t)len, (lnp64_word_t)prot));
}

int munmap(void *addr, size_t len) {
  if (len == 0) {
    lnp64_errno_store(LNP64_EINVAL);
    return -1;
  }
  return lnp64_complete_status((long)__lnp_munmap_bootstrap(addr));
}

int msync(void *addr, size_t len, int flags) {
  (void)addr;
  (void)len;
  (void)flags;
  lnp64_errno_store(LNP64_EINVAL);
  return -1;
}

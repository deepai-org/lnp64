#include "lnp64_intrinsics.h"

typedef unsigned long size_t;

enum { LNP64_AT_FDCWD = -100 };
enum {
  LNP64_FDR_TOKEN_MARKER = 1UL << 62,
  LNP64_FDR_TOKEN_INDEX_MASK = 0xffUL,
};

long openat(int dirfd, const char *path, int flags) {
  lnp64_word_t result =
      __lnp_openat((lnp64_cap_t)(long)dirfd, (lnp64_word_t)path,
                   (lnp64_word_t)flags, 0);
  if ((long)result < 0)
    return (long)result;
  if (result & LNP64_FDR_TOKEN_MARKER)
    return (long)(result & LNP64_FDR_TOKEN_INDEX_MASK);
  return (long)result;
}

long open(const char *path, int flags) {
  return openat(LNP64_AT_FDCWD, path, flags);
}

long read(long fd, void *buf, size_t len) {
  return (long)__lnp_pull((lnp64_cap_t)fd, (lnp64_word_t)buf, len);
}

long write(long fd, const void *buf, size_t len) {
  return (long)__lnp_push((lnp64_cap_t)fd, (lnp64_word_t)buf, len);
}

long lseek(int fd, long offset, int whence) {
  unsigned long result;
  __asm__ volatile("fd_seek_dyn %1, %2, %3\n\tmov %0, r1"
                   : "=r"(result)
                   : "r"((long)fd), "r"(offset), "r"((long)whence)
                   : "memory");
  return (long)result;
}

int close(int fd) {
  return __lnp_cap_revoke((lnp64_cap_t)(long)fd, 0) >= 0 ? 0 : -1;
}

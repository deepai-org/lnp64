#include "lnp64_intrinsics.h"

typedef unsigned long size_t;

enum { LNP64_AT_FDCWD = -100 };

long openat(int dirfd, const char *path, int flags) {
  return (long)__lnp_openat((lnp64_cap_t)(long)dirfd, (lnp64_word_t)path,
                            (lnp64_word_t)flags, 0);
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

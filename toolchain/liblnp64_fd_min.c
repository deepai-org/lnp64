#include "lnp64_intrinsics.h"

typedef unsigned long size_t;

long read(int fd, void *buf, size_t len) {
  return (long)__lnp_pull((lnp64_cap_t)(unsigned long)fd,
                          (lnp64_word_t)buf, len);
}

long write(int fd, const void *buf, size_t len) {
  return (long)__lnp_push((lnp64_cap_t)(unsigned long)fd,
                          (lnp64_word_t)buf, len);
}

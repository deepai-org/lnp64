#include "lnp64_intrinsics.h"

int main(void) {
  unsigned char *buf = (unsigned char *)__lnp_alloc(16);
  if (buf == (void *)(lnp64_word_t)-1) {
    return 2;
  }

  buf[0] = 101;
  buf[8] = 10;
  int result = buf[0] + buf[8] * 1000;
  __lnp_free(buf);

  return result == 10101 ? 0 : 1;
}

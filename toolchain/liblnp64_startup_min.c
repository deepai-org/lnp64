#include "lnp64_intrinsics.h"

enum {
  LNP64_ENV_KEY_PAGE_SIZE = 2,
  LNP64_ENV_KEY_HWCAP0 = 5,
  LNP64_AT_PAGESZ = 6,
  LNP64_AT_HWCAP = 16,
};

static lnp64_word_t lnp64_env_get(lnp64_word_t key, lnp64_word_t index,
                                  lnp64_word_t len) {
  lnp64_word_t result;
  __asm__ volatile("env_get %0, %1, %2, %3"
                   : "=r"(result)
                   : "r"(key), "r"(index), "r"(len)
                   : "memory");
  return result;
}

unsigned long getauxval(unsigned long type) {
  if (type == LNP64_AT_PAGESZ)
    return lnp64_env_get(LNP64_ENV_KEY_PAGE_SIZE, 0, 0);
  if (type == LNP64_AT_HWCAP)
    return lnp64_env_get(LNP64_ENV_KEY_HWCAP0, 0, 0);
  return 0;
}

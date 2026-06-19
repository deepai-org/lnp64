#include "lnp64_intrinsics.h"

void _exit(int status) {
  __lnp_exit((lnp64_word_t)(unsigned int)status);
  __builtin_unreachable();
}

void exit(int status) {
  _exit(status);
}

int pid(void) {
  return (int)__lnp_get_pid();
}

int getpid(void) {
  return pid();
}

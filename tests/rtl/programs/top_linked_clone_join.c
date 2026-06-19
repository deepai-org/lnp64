#include "lnp64_intrinsics.h"

static volatile lnp64_word_t child_marker;

static int child_entry(lnp64_word_t arg) {
  child_marker = arg + 1;
  __lnp_exit(0);
  return 0;
}

int main(void) {
  lnp64_word_t tid = __lnp_spawn_entry((lnp64_word_t)child_entry, 41);
  if (tid == (lnp64_word_t)-1) {
    return 1;
  }
  if (__lnp_thread_join(tid, 0) != 0) {
    return 2;
  }
  if (child_marker != 42) {
    return 3;
  }
  return 0;
}

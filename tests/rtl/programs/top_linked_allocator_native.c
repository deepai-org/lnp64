#include "lnp64_intrinsics.h"

int main(void) {
  void *first = __lnp_alloc(32);
  void *second = __lnp_alloc(16);
  if (first == (void *)(lnp64_word_t)-1 || second == (void *)(lnp64_word_t)-1 ||
      first == second) {
    return 1;
  }

  char message[9];
  message[0] = 'a';
  message[1] = 'l';
  message[2] = 'l';
  message[3] = 'o';
  message[4] = 'c';
  message[5] = ' ';
  message[6] = 'o';
  message[7] = 'k';
  message[8] = '\n';
  __lnp_push(1, (lnp64_word_t)message, sizeof(message));
  __lnp_free(second);
  __lnp_free(first);
  return 0;
}

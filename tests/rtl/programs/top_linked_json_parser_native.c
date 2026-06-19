#include "lnp64_intrinsics.h"

int main(void) {
  lnp64_word_t *nodes = (lnp64_word_t *)__lnp_alloc(sizeof(lnp64_word_t));
  if (nodes == (void *)(lnp64_word_t)-1) {
    return 1;
  }

  *nodes = 0;
  for (lnp64_word_t i = 0; i < 64; ++i) {
    lnp64_word_t *node = (lnp64_word_t *)__lnp_alloc(32);
    if (node == (void *)(lnp64_word_t)-1) {
      return 2;
    }
    node[0] = i;
    *nodes = *nodes + 1;
    __lnp_free(node);
  }

  if (*nodes != 64) {
    return 3;
  }

  char message[15];
  message[0] = 'j';
  message[1] = 's';
  message[2] = 'o';
  message[3] = 'n';
  message[4] = ' ';
  message[5] = 'p';
  message[6] = 'a';
  message[7] = 'r';
  message[8] = 's';
  message[9] = 'e';
  message[10] = 'r';
  message[11] = ' ';
  message[12] = 'o';
  message[13] = 'k';
  message[14] = '\n';
  __lnp_push(1, (lnp64_word_t)message, sizeof(message));
  __lnp_free(nodes);
  return 0;
}

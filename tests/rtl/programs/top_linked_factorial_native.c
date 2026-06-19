#include "lnp64_intrinsics.h"

int main(void) {
  unsigned long n = 5;
  unsigned long acc = 1;
  while (n > 1) {
    acc = acc * n;
    n = n - 1;
  }

  if (acc != 120) {
    return 1;
  }

  char message[13];
  message[0] = 'f';
  message[1] = 'a';
  message[2] = 'c';
  message[3] = 't';
  message[4] = 'o';
  message[5] = 'r';
  message[6] = 'i';
  message[7] = 'a';
  message[8] = 'l';
  message[9] = ' ';
  message[10] = 'o';
  message[11] = 'k';
  message[12] = '\n';
  __lnp_push(1, (lnp64_word_t)message, sizeof(message));
  return 0;
}

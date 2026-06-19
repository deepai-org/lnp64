#include "lnp64_intrinsics.h"

int main(void) {
  unsigned long input = 10;
  unsigned long i = 0;
  unsigned long a = 0;
  unsigned long b = 1;
  while (i < input) {
    unsigned long next = a + b;
    a = b;
    b = next;
    i = i + 1;
  }

  if (a != 55) {
    return 1;
  }

  char message[13];
  message[0] = 'f';
  message[1] = 'i';
  message[2] = 'b';
  message[3] = 'o';
  message[4] = 'n';
  message[5] = 'a';
  message[6] = 'c';
  message[7] = 'c';
  message[8] = 'i';
  message[9] = ' ';
  message[10] = 'o';
  message[11] = 'k';
  message[12] = '\n';
  __lnp_push(1, (lnp64_word_t)message, sizeof(message));
  return 0;
}

#include "lnp64_intrinsics.h"

int main(void) {
  char message[17];
  message[0] = 'h';
  message[1] = 'e';
  message[2] = 'l';
  message[3] = 'l';
  message[4] = 'o';
  message[5] = ' ';
  message[6] = 'f';
  message[7] = 'r';
  message[8] = 'o';
  message[9] = 'm';
  message[10] = ' ';
  message[11] = 'L';
  message[12] = 'N';
  message[13] = 'P';
  message[14] = '6';
  message[15] = '4';
  message[16] = '\n';
  __lnp_push(1, (lnp64_word_t)message, sizeof(message));
  return 0;
}

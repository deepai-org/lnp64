#include "lnp64_intrinsics.h"

int main(void) {
  char *text = (char *)__lnp_alloc(10);
  if (text == (void *)(lnp64_word_t)-1) {
    return 2;
  }

  text[0] = 'e';
  text[1] = 'b';
  text[2] = 'g';
  text[3] = '1';
  text[4] = '3';
  text[5] = ' ';
  text[6] = 'b';
  text[7] = 'x';
  text[8] = '\n';
  text[9] = 0;

  for (unsigned long i = 0; i < 9; ++i) {
    char c = text[i];
    if (c == 'e') {
      c = 'r';
    } else if (c == 'b') {
      c = 'o';
    } else if (c == 'g') {
      c = 't';
    } else if (c == 'x') {
      c = 'k';
    }
    text[i] = c;
  }

  __lnp_push(1, (lnp64_word_t)text, 9);
  __lnp_free(text);
  return 0;
}

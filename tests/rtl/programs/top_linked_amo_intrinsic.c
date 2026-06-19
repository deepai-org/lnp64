#include "lnp64_intrinsics.h"

static volatile lnp64_word_t cell = 7;

int main(void) {
  if (__lnp_amo_add(&cell, 5) != 7)
    return 1;
  if (cell != 12)
    return 2;
  if (__lnp_amo_and(&cell, 10) != 12)
    return 3;
  if (cell != 8)
    return 4;
  if (__lnp_amo_or(&cell, 3) != 8)
    return 5;
  if (cell != 11)
    return 6;
  if (__lnp_amo_xor(&cell, 6) != 11)
    return 7;
  if (cell != 13)
    return 8;
  if (__lnp_amo_swap(&cell, 42) != 13)
    return 9;
  return cell == 42 ? 0 : 10;
}

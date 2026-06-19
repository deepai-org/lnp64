static volatile unsigned long bit_a = 10UL;
static volatile unsigned long bit_b = 12UL;
static volatile unsigned long shift_x = 3UL;
static volatile unsigned long shift_y = 1UL;

__attribute__((noinline)) unsigned long bit_mix(unsigned long x,
                                                unsigned long y) {
  return ((x & y) | (x ^ y)) + (~x & 0xffUL);
}

__attribute__((noinline)) unsigned long shift_mix(unsigned long x,
                                                  unsigned long y) {
  return (x << y) + (x >> y);
}

int main(void) {
  unsigned long result = bit_mix(bit_a, bit_b) + shift_mix(shift_x, shift_y);
  return result == 266UL ? 0 : 1;
}

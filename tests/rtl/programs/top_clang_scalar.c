int main(void) {
  unsigned long a = 6;
  unsigned long b = 7;
  unsigned long c = 0xff;
  __asm__ volatile("" : "+r"(a), "+r"(b), "+r"(c));
  unsigned long acc = (a * b) ^ 42;
  acc |= (c & 0x0f) ^ 15;
  acc |= ((a << 3) >> 3) ^ 6;
  acc |= (b - a) ^ 1;
  return (int)acc;
}

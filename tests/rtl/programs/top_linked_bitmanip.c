static volatile unsigned long clz_input = 0x00f0000000000000UL;
static volatile unsigned long ctz_input = 0x10UL;
static volatile unsigned long pop_input = 0xf0f0UL;
static volatile unsigned long bswap_input = 0x0123456789abcdefUL;
static volatile unsigned long rol_input = 1UL;
static volatile unsigned long ror_input = 0x100UL;
static volatile unsigned long rotate_count = 8UL;

__attribute__((noinline)) unsigned long clz64(unsigned long x) {
  return __builtin_clzll(x);
}

__attribute__((noinline)) unsigned long ctz64(unsigned long x) {
  return __builtin_ctzll(x);
}

__attribute__((noinline)) unsigned long pop64(unsigned long x) {
  return __builtin_popcountll(x);
}

__attribute__((noinline)) unsigned long rol64(unsigned long x, unsigned long n) {
  n &= 63;
  return (x << n) | (x >> ((64 - n) & 63));
}

__attribute__((noinline)) unsigned long ror64(unsigned long x, unsigned long n) {
  n &= 63;
  return (x >> n) | (x << ((64 - n) & 63));
}

__attribute__((noinline)) unsigned long bswap64(unsigned long x) {
  return __builtin_bswap64(x);
}

int main(void) {
  return (int)((clz64(clz_input) - 8) + (ctz64(ctz_input) - 4) +
               (pop64(pop_input) - 8) +
               ((rol64(rol_input, rotate_count) >> 8) - 1) +
               (ror64(ror_input, rotate_count) - 1) +
               ((bswap64(bswap_input) & 0xff) - 1));
}

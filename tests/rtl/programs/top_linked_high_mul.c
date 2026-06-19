static volatile unsigned long uhi = 0xffffffffffffffffUL;
static volatile unsigned long ulo = 2;
static volatile long sneg = -2;
static volatile long spos = 3;

__attribute__((noinline)) unsigned long umul_high(unsigned long a,
                                                  unsigned long b) {
  return ((unsigned __int128)a * (unsigned __int128)b) >> 64;
}

__attribute__((noinline)) long smul_high(long a, long b) {
  return ((__int128)a * (__int128)b) >> 64;
}

int main(void) {
  return (int)((umul_high(uhi, ulo) - 1) + (smul_high(sneg, spos) + 1));
}

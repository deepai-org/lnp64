static volatile unsigned long ua = 100UL;
static volatile unsigned long ub = 7UL;
static volatile long sa = -100L;
static volatile long sb = 7L;

__attribute__((noinline)) unsigned long uquot(unsigned long a, unsigned long b) {
  return a / b;
}

__attribute__((noinline)) unsigned long urem64(unsigned long a, unsigned long b) {
  return a % b;
}

__attribute__((noinline)) long squot(long a, long b) {
  return a / b;
}

__attribute__((noinline)) long srem64(long a, long b) {
  return a % b;
}

int main(void) {
  return (int)((uquot(ua, ub) - 14UL) + (urem64(ua, ub) - 2UL) +
               (squot(sa, sb) + 14L) + (srem64(sa, sb) + 2L));
}

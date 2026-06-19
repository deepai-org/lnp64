static volatile unsigned long bss_counter;
static volatile unsigned long data_seed = 11;

__attribute__((noinline)) unsigned long stack_bss_mix(unsigned long limit) {
  volatile unsigned long scratch[4];
  unsigned long initial_bss = bss_counter;
  scratch[0] = data_seed;
  scratch[1] = initial_bss;
  scratch[2] = 0;
  scratch[3] = 1;

  for (unsigned long i = 0; i < limit; ++i) {
    scratch[i & 3] += i + data_seed;
  }

  bss_counter = scratch[0] + scratch[1] + scratch[2] + scratch[3];
  return bss_counter | (initial_bss << 63);
}

int main(void) {
  unsigned long got = stack_bss_mix(6);
  if ((got >> 63) != 0) {
    return 2;
  }
  if (got == 0) {
    return 3;
  }
  if (bss_counter != got) {
    return 4;
  }
  if (data_seed != 11) {
    return 5;
  }
  return 0;
}

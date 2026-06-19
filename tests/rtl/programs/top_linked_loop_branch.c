static volatile unsigned long limit_value = 9;
static volatile unsigned long bias_value = 3;

__attribute__((noinline)) unsigned long branch_mix(unsigned long limit) {
  unsigned long acc = 0;
  for (unsigned long i = 0; i < limit; ++i) {
    if ((i & 1) == 0) {
      acc += i + bias_value;
    } else {
      acc += i * 2;
    }
  }
  return acc;
}

int main(void) {
  unsigned long got = branch_mix(limit_value);
  return (int)(got - 54);
}

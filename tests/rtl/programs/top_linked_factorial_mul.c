static volatile unsigned long factorial_limit = 5UL;

__attribute__((noinline)) unsigned long mul_step(unsigned long acc,
                                                 unsigned long value) {
  return acc * value;
}

__attribute__((noinline)) unsigned long factorial(unsigned long limit) {
  unsigned long acc = 1;
  for (unsigned long i = 1; i <= limit; ++i) {
    acc = mul_step(acc, i);
  }
  return acc;
}

int main(void) {
  return factorial(factorial_limit) == 120UL ? 0 : 1;
}

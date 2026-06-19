static unsigned long cell = 7;

int main(void) {
  unsigned long loaded = __atomic_load_n(&cell, __ATOMIC_SEQ_CST);
  if (loaded != 7)
    return 1;

  __atomic_store_n(&cell, 9, __ATOMIC_SEQ_CST);
  if (__atomic_load_n(&cell, __ATOMIC_SEQ_CST) != 9)
    return 2;

  unsigned long old_add = __atomic_fetch_add(&cell, 3, __ATOMIC_SEQ_CST);
  if (old_add != 9 || cell != 12)
    return 3;

  unsigned long old_and = __atomic_fetch_and(&cell, 10, __ATOMIC_SEQ_CST);
  if (old_and != 12 || cell != 8)
    return 4;

  unsigned long old_or = __atomic_fetch_or(&cell, 3, __ATOMIC_SEQ_CST);
  if (old_or != 8 || cell != 11)
    return 5;

  unsigned long old_xor = __atomic_fetch_xor(&cell, 6, __ATOMIC_SEQ_CST);
  if (old_xor != 11 || cell != 13)
    return 6;

  unsigned long old_swap = __atomic_exchange_n(&cell, 42, __ATOMIC_SEQ_CST);
  if (old_swap != 13 || cell != 42)
    return 7;

  unsigned long expected = 42;
  int exchanged = __atomic_compare_exchange_n(&cell, &expected, 99, 0,
                                              __ATOMIC_SEQ_CST,
                                              __ATOMIC_SEQ_CST);
  if (!exchanged || expected != 42 || cell != 99)
    return 8;

  expected = 42;
  exchanged = __atomic_compare_exchange_n(&cell, &expected, 123, 0,
                                          __ATOMIC_SEQ_CST,
                                          __ATOMIC_SEQ_CST);
  if (exchanged || expected != 99 || cell != 99)
    return 9;

  return 0;
}

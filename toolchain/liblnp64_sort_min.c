typedef unsigned long size_t;

static void lnp64_swap_bytes(char *lhs, char *rhs, size_t width) {
  for (size_t i = 0; i < width; i = i + 1) {
    char tmp = lhs[i];
    lhs[i] = rhs[i];
    rhs[i] = tmp;
  }
}

void qsort(void *base, size_t nmemb, size_t width,
           int (*compar)(const void *, const void *)) {
  char *items = (char *)base;
  if (!base || width == 0 || nmemb < 2)
    return;

  for (size_t i = 1; i < nmemb; i = i + 1) {
    size_t j = i;
    while (j > 0) {
      char *prev = items + (j - 1) * width;
      char *cur = items + j * width;
      if (compar(prev, cur) <= 0)
        break;
      lnp64_swap_bytes(prev, cur, width);
      j = j - 1;
    }
  }
}

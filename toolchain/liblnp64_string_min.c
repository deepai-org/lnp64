typedef unsigned long size_t;

size_t strlen(const char *s) {
  size_t n = 0;
  while (s[n] != 0)
    n = n + 1;
  return n;
}

void *memcpy(void *dst, const void *src, size_t len) {
  unsigned char *d = dst;
  const unsigned char *s = src;
  for (size_t i = 0; i < len; i = i + 1)
    d[i] = s[i];
  return dst;
}

void *memmove(void *dst, const void *src, size_t len) {
  unsigned char *d = dst;
  const unsigned char *s = src;
  if (d <= s) {
    for (size_t i = 0; i < len; i = i + 1)
      d[i] = s[i];
  } else {
    for (size_t i = len; i != 0; i = i - 1)
      d[i - 1] = s[i - 1];
  }
  return dst;
}

int memcmp(const void *lhs, const void *rhs, size_t len) {
  const unsigned char *a = lhs;
  const unsigned char *b = rhs;
  for (size_t i = 0; i < len; i = i + 1) {
    if (a[i] != b[i])
      return (int)a[i] - (int)b[i];
  }
  return 0;
}

void *memset(void *dst, int value, size_t len) {
  unsigned char *d = dst;
  for (size_t i = 0; i < len; i = i + 1)
    d[i] = (unsigned char)value;
  return dst;
}

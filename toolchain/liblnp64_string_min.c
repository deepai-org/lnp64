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

int strcmp(const char *lhs, const char *rhs) {
  const unsigned char *a = (const unsigned char *)lhs;
  const unsigned char *b = (const unsigned char *)rhs;
  while (*a != 0 && *a == *b) {
    a = a + 1;
    b = b + 1;
  }
  return (int)*a - (int)*b;
}

int strncmp(const char *lhs, const char *rhs, size_t len) {
  const unsigned char *a = (const unsigned char *)lhs;
  const unsigned char *b = (const unsigned char *)rhs;
  for (size_t i = 0; i < len; i = i + 1) {
    if (a[i] == 0 || a[i] != b[i])
      return (int)a[i] - (int)b[i];
  }
  return 0;
}

char *strcpy(char *dst, const char *src) {
  size_t i = 0;
  do {
    dst[i] = src[i];
    i = i + 1;
  } while (dst[i - 1] != 0);
  return dst;
}

char *strncpy(char *dst, const char *src, size_t len) {
  size_t i = 0;
  while (i < len && src[i] != 0) {
    dst[i] = src[i];
    i = i + 1;
  }
  while (i < len) {
    dst[i] = 0;
    i = i + 1;
  }
  return dst;
}

char *strncat(char *dst, const char *src, size_t len) {
  size_t d = strlen(dst);
  size_t i = 0;
  while (i < len && src[i] != 0) {
    dst[d + i] = src[i];
    i = i + 1;
  }
  dst[d + i] = 0;
  return dst;
}

char *strchr(const char *s, int ch) {
  unsigned char needle = (unsigned char)ch;
  for (;; s = s + 1) {
    if ((unsigned char)*s == needle)
      return (char *)s;
    if (*s == 0)
      return 0;
  }
}

char *strrchr(const char *s, int ch) {
  unsigned char needle = (unsigned char)ch;
  const char *last = 0;
  for (;; s = s + 1) {
    if ((unsigned char)*s == needle)
      last = s;
    if (*s == 0)
      return (char *)last;
  }
}

char *strstr(const char *haystack, const char *needle) {
  if (needle[0] == 0)
    return (char *)haystack;
  for (const char *h = haystack; *h != 0; h = h + 1) {
    size_t i = 0;
    while (needle[i] != 0 && h[i] == needle[i])
      i = i + 1;
    if (needle[i] == 0)
      return (char *)h;
  }
  return 0;
}

static int lnp64_byte_in_set(unsigned char ch, const char *set) {
  for (const unsigned char *s = (const unsigned char *)set; *s != 0;
       s = s + 1) {
    if (*s == ch)
      return 1;
  }
  return 0;
}

size_t strspn(const char *s, const char *accept) {
  size_t n = 0;
  while (s[n] != 0 && lnp64_byte_in_set((unsigned char)s[n], accept))
    n = n + 1;
  return n;
}

size_t strcspn(const char *s, const char *reject) {
  size_t n = 0;
  while (s[n] != 0 && !lnp64_byte_in_set((unsigned char)s[n], reject))
    n = n + 1;
  return n;
}

char *strpbrk(const char *s, const char *accept) {
  for (; *s != 0; s = s + 1) {
    if (lnp64_byte_in_set((unsigned char)*s, accept))
      return (char *)s;
  }
  return 0;
}

static char *lnp64_strtok_next;

char *strtok(char *s, const char *delim) {
  if (s == 0)
    s = lnp64_strtok_next;
  if (s == 0)
    return 0;
  while (*s != 0 && lnp64_byte_in_set((unsigned char)*s, delim))
    s = s + 1;
  if (*s == 0) {
    lnp64_strtok_next = 0;
    return 0;
  }
  char *token = s;
  while (*s != 0 && !lnp64_byte_in_set((unsigned char)*s, delim))
    s = s + 1;
  if (*s == 0) {
    lnp64_strtok_next = 0;
  } else {
    *s = 0;
    lnp64_strtok_next = s + 1;
  }
  return token;
}

size_t strlcpy(char *dst, const char *src, size_t size) {
  size_t src_len = strlen(src);
  if (size != 0) {
    size_t i = 0;
    while (i + 1 < size && src[i] != 0) {
      dst[i] = src[i];
      i = i + 1;
    }
    dst[i] = 0;
  }
  return src_len;
}

size_t strlcat(char *dst, const char *src, size_t size) {
  size_t dst_len = 0;
  while (dst_len < size && dst[dst_len] != 0)
    dst_len = dst_len + 1;
  size_t src_len = strlen(src);
  if (dst_len == size)
    return size + src_len;
  size_t i = 0;
  while (dst_len + i + 1 < size && src[i] != 0) {
    dst[dst_len + i] = src[i];
    i = i + 1;
  }
  dst[dst_len + i] = 0;
  return dst_len + src_len;
}

void *memmem(const void *haystack, size_t haystack_len, const void *needle,
             size_t needle_len) {
  const unsigned char *h = haystack;
  const unsigned char *n = needle;
  if (needle_len == 0)
    return (void *)haystack;
  if (needle_len > haystack_len)
    return 0;
  for (size_t i = 0; i + needle_len <= haystack_len; i = i + 1) {
    size_t j = 0;
    while (j < needle_len && h[i + j] == n[j])
      j = j + 1;
    if (j == needle_len)
      return (void *)(h + i);
  }
  return 0;
}

int islower(int ch) { return ch >= 'a' && ch <= 'z'; }

int isupper(int ch) { return ch >= 'A' && ch <= 'Z'; }

int isalpha(int ch) { return islower(ch) || isupper(ch); }

int isdigit(int ch) { return ch >= '0' && ch <= '9'; }

int isalnum(int ch) { return isalpha(ch) || isdigit(ch); }

int isspace(int ch) {
  return ch == ' ' || (ch >= '\t' && ch <= '\r');
}

int isxdigit(int ch) {
  return isdigit(ch) || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F');
}

int tolower(int ch) {
  if (isupper(ch))
    return ch + ('a' - 'A');
  return ch;
}

int toupper(int ch) {
  if (islower(ch))
    return ch - ('a' - 'A');
  return ch;
}

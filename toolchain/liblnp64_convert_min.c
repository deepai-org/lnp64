#include <errno.h>
#include <stdlib.h>

int lnp64_errno_store(int value);

#define LNP64_NOINLINE __attribute__((noinline))

int abs(int value) { return value < 0 ? -value : value; }

long labs(long value) { return value < 0 ? -value : value; }

static int lnp64_isspace(int ch) {
  return ch == ' ' || (ch >= '\t' && ch <= '\r');
}

static int lnp64_digit_value(int ch) {
  if (ch >= '0' && ch <= '9')
    return ch - '0';
  if (ch >= 'a' && ch <= 'z')
    return ch - 'a' + 10;
  if (ch >= 'A' && ch <= 'Z')
    return ch - 'A' + 10;
  return -1;
}

static int lnp64_has_digit_for_base(const char *s, int base) {
  int digit = lnp64_digit_value((unsigned char)*s);
  return digit >= 0 && digit < base;
}

static LNP64_NOINLINE unsigned long long
lnp64_u64_from_halves(unsigned long hi, unsigned long lo) {
  unsigned long long value = hi;
  value = value << 16;
  value = value << 16;
  return value | lo;
}

static LNP64_NOINLINE unsigned long long lnp64_signed_max_abs(void) {
  return lnp64_u64_from_halves(0x7fffffffUL, 0xffffffffUL);
}

static LNP64_NOINLINE unsigned long long lnp64_signed_min_abs(void) {
  return lnp64_u64_from_halves(0x80000000UL, 0);
}

static const char *lnp64_parse_prefix(const char *s, int *base) {
  if (*base == 0) {
    if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X') &&
        lnp64_has_digit_for_base(s + 2, 16)) {
      *base = 16;
      return s + 2;
    }
    if (s[0] == '0') {
      *base = 8;
      return s;
    }
    *base = 10;
    return s;
  }
  if (*base == 16 && s[0] == '0' && (s[1] == 'x' || s[1] == 'X') &&
      lnp64_has_digit_for_base(s + 2, 16))
    return s + 2;
  return s;
}

static unsigned long long lnp64_parse_unsigned(const char *nptr,
                                               char **endptr,
                                               int base,
                                               unsigned long long limit,
                                               int *negative,
                                               int *overflow) {
  const char *start = nptr;
  const char *s = nptr;
  unsigned long long value = 0;
  int any = 0;

  *negative = 0;
  *overflow = 0;
  while (lnp64_isspace((unsigned char)*s))
    s = s + 1;
  if (*s == '+' || *s == '-') {
    *negative = *s == '-';
    s = s + 1;
  }
  if (base != 0 && (base < 2 || base > 36)) {
    lnp64_errno_store(EINVAL);
    if (endptr)
      *endptr = (char *)start;
    return 0;
  }
  s = lnp64_parse_prefix(s, &base);
  for (;;) {
    int digit = lnp64_digit_value((unsigned char)*s);
    if (digit < 0 || digit >= base)
      break;
    any = 1;
    if (value > (limit - (unsigned long long)digit) / (unsigned long long)base) {
      *overflow = 1;
    } else if (!*overflow) {
      value = value * (unsigned long long)base + (unsigned long long)digit;
    }
    s = s + 1;
  }
  if (endptr)
    *endptr = (char *)(any ? s : start);
  return value;
}

unsigned long long strtoull(const char *nptr, char **endptr, int base) {
  int negative;
  int overflow;
  unsigned long long value =
      lnp64_parse_unsigned(nptr, endptr, base, ~0ULL, &negative, &overflow);
  if (overflow) {
    lnp64_errno_store(ERANGE);
    return ~0ULL;
  }
  if (negative)
    return 0ULL - value;
  return value;
}

unsigned long strtoul(const char *nptr, char **endptr, int base) {
  return (unsigned long)strtoull(nptr, endptr, base);
}

long long strtoll(const char *nptr, char **endptr, int base) {
  int negative;
  int overflow;
  unsigned long long limit = lnp64_signed_max_abs();
  unsigned long long value;
  if (nptr) {
    const char *s = nptr;
    while (lnp64_isspace((unsigned char)*s))
      s = s + 1;
    if (*s == '-')
      limit = lnp64_signed_min_abs();
  }
  value = lnp64_parse_unsigned(nptr, endptr, base, limit, &negative, &overflow);
  if (overflow) {
    lnp64_errno_store(ERANGE);
    if (negative)
      return (long long)lnp64_signed_min_abs();
    return (long long)lnp64_signed_max_abs();
  }
  if (negative) {
    if (value == lnp64_signed_min_abs())
      return (long long)lnp64_signed_min_abs();
    return -(long long)value;
  }
  return (long long)value;
}

long strtol(const char *nptr, char **endptr, int base) {
  return (long)strtoll(nptr, endptr, base);
}

long atol(const char *nptr) {
  return strtol(nptr, 0, 10);
}

long long atoll(const char *nptr) {
  return strtoll(nptr, 0, 10);
}

int atoi(const char *nptr) {
  return (int)strtol(nptr, 0, 10);
}

double strtod(const char *nptr, char **endptr) {
  if (endptr)
    *endptr = (char *)nptr;
  return 0.0;
}

/* Weak so a full soft-float runtime (liblnp64_softfloat_min) overrides these
 * when linked. Standalone convert (used by packages without soft-float) still
 * gets a correct ordered comparison rather than the previous return-0 stub. */
static int lnp64_convert_dcmp(double a, double b) {
  union {
    double d;
    unsigned long u;
  } x, y;
  x.d = a;
  y.d = b;
  unsigned long am = x.u & 0x7fffffffffffffffUL;
  unsigned long bm = y.u & 0x7fffffffffffffffUL;
  if (am > 0x7ff0000000000000UL || bm > 0x7ff0000000000000UL)
    return 2; /* unordered (NaN) */
  if (am == 0 && bm == 0)
    return 0;
  if (x.u == y.u)
    return 0;
  int as = (int)(x.u >> 63), bs = (int)(y.u >> 63);
  if (as != bs)
    return as ? -1 : 1;
  if (!as)
    return am < bm ? -1 : 1;
  return am > bm ? -1 : 1;
}

__attribute__((weak)) int __ltdf2(double lhs, double rhs) {
  int c = lnp64_convert_dcmp(lhs, rhs);
  return c == 2 ? 1 : c;
}

__attribute__((weak)) int __gtdf2(double lhs, double rhs) {
  int c = lnp64_convert_dcmp(lhs, rhs);
  return c == 2 ? -1 : c;
}

/* strtold: LNP64 has no 80-bit extended float; map to double */
double strtold(const char *nptr, char **endptr) {
  return strtod(nptr, endptr);
}

void *bsearch(const void *key, const void *base, size_t nmemb, size_t size,
              int (*cmp)(const void *, const void *)) {
  size_t lo = 0, hi = nmemb;
  while (lo < hi) {
    size_t mid = lo + (hi - lo) / 2;
    int r = cmp(key, (const char *)base + mid * size);
    if (r == 0) return (void *)((const char *)base + mid * size);
    if (r < 0) hi = mid; else lo = mid + 1;
  }
  return 0;
}
int atexit(void (*function)(void)) { (void)function; return 0; }
char *realpath(const char *path, char *resolved) {
  if (!resolved) return 0;
  size_t len = 0;
  while (path[len]) len++;
  if (len >= 4096) return 0;
  for (size_t i = 0; i <= len; i++) resolved[i] = path[i];
  return resolved;
}

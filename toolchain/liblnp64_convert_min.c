enum {
  LNP64_EINVAL = 22,
  LNP64_ERANGE = 34,
};

int lnp64_errno_store(int value);

#define LNP64_NOINLINE __attribute__((noinline))

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
    lnp64_errno_store(LNP64_EINVAL);
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
    lnp64_errno_store(LNP64_ERANGE);
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
    lnp64_errno_store(LNP64_ERANGE);
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

int atoi(const char *nptr) {
  return (int)strtol(nptr, 0, 10);
}

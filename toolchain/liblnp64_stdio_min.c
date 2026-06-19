#include <stdarg.h>
#include <stddef.h>

struct lnp64_snprintf_out {
  char *buf;
  size_t size;
  size_t len;
};

static void lnp64_out_char(struct lnp64_snprintf_out *out, char ch) {
  if (out->size != 0 && out->len + 1 < out->size)
    out->buf[out->len] = ch;
  out->len = out->len + 1;
}

static void lnp64_out_string(struct lnp64_snprintf_out *out, const char *s) {
  if (!s)
    s = "(null)";
  while (*s) {
    lnp64_out_char(out, *s);
    s = s + 1;
  }
}

static void lnp64_out_unsigned(struct lnp64_snprintf_out *out,
                               unsigned long long value, unsigned base) {
  char tmp[32];
  size_t used = 0;
  const char *digits = "0123456789abcdef";
  if (value == 0) {
    lnp64_out_char(out, '0');
    return;
  }
  while (value != 0 && used < sizeof tmp) {
    tmp[used] = digits[value % base];
    value = value / base;
    used = used + 1;
  }
  while (used != 0) {
    used = used - 1;
    lnp64_out_char(out, tmp[used]);
  }
}

static void lnp64_out_signed(struct lnp64_snprintf_out *out, long long value) {
  unsigned long long magnitude;
  if (value < 0) {
    lnp64_out_char(out, '-');
    magnitude = (unsigned long long)(-(value + 1)) + 1;
  } else {
    magnitude = (unsigned long long)value;
  }
  lnp64_out_unsigned(out, magnitude, 10);
}

int vsnprintf(char *str, unsigned long size, const char *format, va_list ap) {
  struct lnp64_snprintf_out out = {str, size, 0};
  while (*format) {
    if (*format != '%') {
      lnp64_out_char(&out, *format);
      format = format + 1;
      continue;
    }

    format = format + 1;
    if (*format == '%') {
      lnp64_out_char(&out, '%');
      format = format + 1;
      continue;
    }

    int long_count = 0;
    while (*format == 'l') {
      long_count = long_count + 1;
      format = format + 1;
    }

    switch (*format) {
    case 's':
      lnp64_out_string(&out, va_arg(ap, const char *));
      break;
    case 'c':
      lnp64_out_char(&out, (char)va_arg(ap, int));
      break;
    case 'd':
    case 'i':
      if (long_count >= 2)
        lnp64_out_signed(&out, va_arg(ap, long long));
      else if (long_count == 1)
        lnp64_out_signed(&out, va_arg(ap, long));
      else
        lnp64_out_signed(&out, va_arg(ap, int));
      break;
    case 'u':
      if (long_count >= 2)
        lnp64_out_unsigned(&out, va_arg(ap, unsigned long long), 10);
      else if (long_count == 1)
        lnp64_out_unsigned(&out, va_arg(ap, unsigned long), 10);
      else
        lnp64_out_unsigned(&out, va_arg(ap, unsigned int), 10);
      break;
    case 'x':
      if (long_count >= 2)
        lnp64_out_unsigned(&out, va_arg(ap, unsigned long long), 16);
      else if (long_count == 1)
        lnp64_out_unsigned(&out, va_arg(ap, unsigned long), 16);
      else
        lnp64_out_unsigned(&out, va_arg(ap, unsigned int), 16);
      break;
    case 'p':
      lnp64_out_string(&out, "0x");
      lnp64_out_unsigned(&out, (unsigned long)va_arg(ap, void *), 16);
      break;
    case 0:
      format = format - 1;
      break;
    default:
      lnp64_out_char(&out, '%');
      lnp64_out_char(&out, *format);
      break;
    }
    if (*format)
      format = format + 1;
  }

  if (size != 0) {
    size_t nul = out.len < size ? out.len : size - 1;
    str[nul] = 0;
  }
  return (int)out.len;
}

int snprintf(char *str, unsigned long size, const char *format, ...) {
  va_list ap;
  int ret;
  va_start(ap, format);
  ret = vsnprintf(str, size, format, ap);
  va_end(ap);
  return ret;
}

int sprintf(char *str, const char *format, ...) {
  va_list ap;
  int ret;
  va_start(ap, format);
  ret = vsnprintf(str, (unsigned long)-1, format, ap);
  va_end(ap);
  return ret;
}

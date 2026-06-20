#include <fcntl.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <unistd.h>

#include "../third_party/sbase/utf.h"

struct __lnp64_file {
  int fd;
};

static FILE lnp64_wc_files[4];
static int lnp64_wc_file_used[4];

static void lnp64_print_size(size_t value) {
  char buf[32];
  int pos = 0;
  do {
    buf[pos++] = (char)('0' + (value % 10));
    value /= 10;
  } while (value);
  while (pos)
    putchar(buf[--pos]);
}

int printf(const char *format, ...) {
  va_list ap;
  int written = 0;
  va_start(ap, format);
  while (*format) {
    if (*format != '%') {
      putchar((unsigned char)*format++);
      written++;
      continue;
    }
    format++;
    if (*format == '%') {
      putchar('%');
      written++;
      format++;
      continue;
    }
    if (*format == 's') {
      const char *s = va_arg(ap, const char *);
      if (!s)
        s = "(null)";
      while (*s) {
        putchar((unsigned char)*s++);
        written++;
      }
      format++;
      continue;
    }
    if (*format == 'z' && format[1] == 'u') {
      size_t value = va_arg(ap, size_t);
      lnp64_print_size(value);
      do {
        written++;
        value /= 10;
      } while (value);
      format += 2;
      continue;
    }
    putchar('%');
    written++;
  }
  va_end(ap);
  return written;
}

FILE *fopen(const char *path, const char *mode) {
  int flags = O_RDONLY;
  if (mode && mode[0] == 'w')
    flags = O_WRONLY | O_CREAT | O_TRUNC;
  int fd = open(path, flags, 0666);
  if (fd < 0)
    return 0;
  for (size_t i = 0; i < sizeof(lnp64_wc_files) / sizeof(lnp64_wc_files[0]);
       i++) {
    if (!lnp64_wc_file_used[i]) {
      lnp64_wc_file_used[i] = 1;
      lnp64_wc_files[i].fd = fd;
      return &lnp64_wc_files[i];
    }
  }
  close(fd);
  return 0;
}

int fgetc(FILE *stream) {
  unsigned char ch;
  if (!stream)
    return EOF;
  return read(stream->fd, &ch, 1) == 1 ? ch : EOF;
}

int fgetrune(Rune *r, FILE *fp) {
  int ch = fgetc(fp);
  if (ch == EOF)
    return 0;
  *r = (ch < Runeself) ? ch : Runeerror;
  return 1;
}

int efgetrune(Rune *r, FILE *fp, const char *str) {
  (void)str;
  return fgetrune(r, fp);
}

int isspacerune(Rune r) {
  return r == ' ' || r == '\t' || r == '\n' || r == '\r' || r == '\f' ||
         r == '\v';
}

#include <dirent.h>
#include <errno.h>
#include <grp.h>
#include <pwd.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "../third_party/sbase/utf.h"
#include "../third_party/sbase/util.h"

static struct dirent lnp64_dirent;
static char lnp64_humansize_buf[32];

static void lnp64_out_char(char **out, size_t *remaining, int *count, int ch) {
  if (*remaining > 1) {
    **out = (char)ch;
    *out = *out + 1;
    *remaining = *remaining - 1;
  }
  *count = *count + 1;
}

static void lnp64_out_string(char **out, size_t *remaining, int *count,
                             const char *s) {
  if (!s)
    s = "(null)";
  while (*s) {
    lnp64_out_char(out, remaining, count, *s);
    s++;
  }
}

static void lnp64_out_unsigned(char **out, size_t *remaining, int *count,
                               unsigned long long value) {
  char buf[32];
  size_t len = 0;
  do {
    buf[len++] = (char)('0' + value % 10);
    value /= 10;
  } while (value);
  while (len)
    lnp64_out_char(out, remaining, count, buf[--len]);
}

static void lnp64_out_signed(char **out, size_t *remaining, int *count,
                             long long value) {
  if (value < 0) {
    lnp64_out_char(out, remaining, count, '-');
    value = -value;
  }
  lnp64_out_unsigned(out, remaining, count, (unsigned long long)value);
}

int vsnprintf(char *str, size_t size, const char *format, va_list ap) {
  char discard = 0;
  char *out = str ? str : &discard;
  size_t remaining = str ? size : 0;
  int count = 0;

  while (*format) {
    int long_count = 0;
    if (*format != '%') {
      lnp64_out_char(&out, &remaining, &count, *format++);
      continue;
    }
    format++;
    if (*format == '%') {
      lnp64_out_char(&out, &remaining, &count, *format++);
      continue;
    }
    while (*format == '-' || *format == '.' ||
           (*format >= '0' && *format <= '9'))
      format++;
    while (*format == 'l') {
      long_count++;
      format++;
    }
    switch (*format++) {
    case 's':
      lnp64_out_string(&out, &remaining, &count, va_arg(ap, const char *));
      break;
    case 'd':
    case 'i':
      if (long_count >= 2)
        lnp64_out_signed(&out, &remaining, &count, va_arg(ap, long long));
      else if (long_count == 1)
        lnp64_out_signed(&out, &remaining, &count, va_arg(ap, long));
      else
        lnp64_out_signed(&out, &remaining, &count, va_arg(ap, int));
      break;
    case 'u':
      if (long_count >= 2)
        lnp64_out_unsigned(&out, &remaining, &count,
                           va_arg(ap, unsigned long long));
      else if (long_count == 1)
        lnp64_out_unsigned(&out, &remaining, &count,
                           va_arg(ap, unsigned long));
      else
        lnp64_out_unsigned(&out, &remaining, &count, va_arg(ap, unsigned int));
      break;
    default:
      lnp64_out_char(&out, &remaining, &count, '?');
      break;
    }
  }

  if (str && size) {
    if (remaining)
      *out = '\0';
    else
      str[size - 1] = '\0';
  }
  return count;
}

int snprintf(char *str, size_t size, const char *format, ...) {
  int ret;
  va_list ap;
  va_start(ap, format);
  ret = vsnprintf(str, size, format, ap);
  va_end(ap);
  return ret;
}

int printf(const char *format, ...) {
  char buf[256];
  int ret;
  va_list ap;
  va_start(ap, format);
  ret = vsnprintf(buf, sizeof(buf), format, ap);
  va_end(ap);
  fputs(buf, stdout);
  return ret;
}

void *reallocarray(void *ptr, size_t count, size_t size) {
  if (size && count > SIZE_MAX / size) {
    errno = ENOMEM;
    return 0;
  }
  return realloc(ptr, count * size);
}

void *ereallocarray(void *ptr, size_t count, size_t size) {
  void *next = reallocarray(ptr, count, size);
  if (!next)
    eprintf("reallocarray:");
  return next;
}

char *estrdup(const char *s) {
  size_t len = strlen(s) + 1;
  char *copy = malloc(len);
  if (!copy)
    eprintf("strdup:");
  memcpy(copy, s, len);
  return copy;
}

struct dirent *readdir(DIR *dirp) {
  unsigned long status;
  memset(&lnp64_dirent, 0, sizeof(lnp64_dirent));
  __asm__ volatile("readdir_fd_dyn %1, %2\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)dirp), "r"(lnp64_dirent.d_name)
                   : "memory");
  if (status == 1)
    return &lnp64_dirent;
  return 0;
}

struct passwd *getpwnam(const char *name) {
  (void)name;
  return 0;
}

struct passwd *getpwuid(uid_t uid) {
  (void)uid;
  return 0;
}

struct group *getgrnam(const char *name) {
  (void)name;
  return 0;
}

struct group *getgrgid(gid_t gid) {
  (void)gid;
  return 0;
}

size_t strftime(char *s, size_t max, const char *format, const struct tm *tm) {
  (void)format;
  if (!s || !max || !tm)
    return 0;
  return (size_t)snprintf(s, max, "%02d:%02d", tm->tm_hour, tm->tm_min);
}

char *humansize(off_t size) {
  snprintf(lnp64_humansize_buf, sizeof(lnp64_humansize_buf), "%lu",
           (unsigned long)size);
  return lnp64_humansize_buf;
}

int chartorune(Rune *r, const char *s) {
  unsigned char ch = s ? (unsigned char)*s : 0;
  if (r)
    *r = ch;
  return 1;
}

int isprintrune(Rune r) {
  return r >= 32 && r < 127;
}

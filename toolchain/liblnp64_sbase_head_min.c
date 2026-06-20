#include <dirent.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef int Rune;

struct __lnp64_file {
  int fd;
  int eof;
  int error;
};

static FILE lnp64_head_stdin_file = {.fd = 0};
static FILE lnp64_head_stdout_file = {.fd = 1};
static FILE lnp64_head_stderr_file = {.fd = 2};
static FILE lnp64_head_files[4];
static int lnp64_head_file_used[4];
static struct dirent lnp64_head_dirent;

FILE *stdin = &lnp64_head_stdin_file;
FILE *stdout = &lnp64_head_stdout_file;
FILE *stderr = &lnp64_head_stderr_file;

char *argv0;

void eprintf(const char *format, ...);

void *erealloc(void *ptr, size_t size) {
  void *next = realloc(ptr, size);
  if (!next && size)
    eprintf("realloc:");
  return next;
}

void *ecalloc(size_t count, size_t size) {
  void *ptr = calloc(count, size);
  if (!ptr)
    eprintf("calloc:");
  return ptr;
}

void *ereallocarray(void *ptr, size_t count, size_t size) {
  if (size && count > ((size_t)-1) / size)
    eprintf("reallocarray:");
  void *next = realloc(ptr, count * size);
  if (!next && count && size)
    eprintf("reallocarray:");
  return next;
}

int charntorune(Rune *r, const char *s, size_t n) {
  if (!r || !s || n == 0)
    return 0;
  unsigned char ch = (unsigned char)s[0];
  *r = ch < 0x80 ? (Rune)ch : 0xfffd;
  return 1;
}

int chartorune(Rune *r, const char *s) { return charntorune(r, s, 1); }

int runetochar(char *s, const Rune *r) {
  if (!s || !r)
    return 0;
  s[0] = (*r >= 0 && *r < 0x80) ? (char)*r : '?';
  return 1;
}

size_t utflen(const char *s) { return strlen(s); }

size_t utftorunestr(const char *s, Rune *r) {
  size_t len = 0;
  while (s[len]) {
    r[len] = (unsigned char)s[len];
    len++;
  }
  r[len] = 0;
  return len;
}

static int lnp64_isalpha(Rune r) {
  return (r >= 'A' && r <= 'Z') || (r >= 'a' && r <= 'z');
}

static int lnp64_isdigit(Rune r) { return r >= '0' && r <= '9'; }

int isalpharune(Rune r) { return lnp64_isalpha(r); }

int isdigitrune(Rune r) { return lnp64_isdigit(r); }

int isalnumrune(Rune r) { return lnp64_isalpha(r) || lnp64_isdigit(r); }

int isblankrune(Rune r) { return r == ' ' || r == '\t'; }

int iscntrlrune(Rune r) { return (r >= 0 && r < 0x20) || r == 0x7f; }

int isgraphrune(Rune r) { return r > ' ' && r < 0x7f; }

int islowerrune(Rune r) { return r >= 'a' && r <= 'z'; }

int isprintrune(Rune r) { return r >= ' ' && r < 0x7f; }

int ispunctrune(Rune r) {
  return isgraphrune(r) && !isalnumrune(r);
}

int isspacerune(Rune r) {
  return r == ' ' || r == '\t' || r == '\n' || r == '\r' || r == '\f' ||
         r == '\v';
}

int isupperrune(Rune r) { return r >= 'A' && r <= 'Z'; }

int isxdigitrune(Rune r) {
  return lnp64_isdigit(r) || (r >= 'A' && r <= 'F') || (r >= 'a' && r <= 'f');
}

Rune tolowerrune(Rune r) {
  return isupperrune(r) ? r + ('a' - 'A') : r;
}

Rune toupperrune(Rune r) {
  return islowerrune(r) ? r - ('a' - 'A') : r;
}

static int lnp64_file_fd(FILE *stream) {
  if (!stream)
    return -1;
  return stream->fd;
}

int fflush(FILE *stream) {
  (void)stream;
  return 0;
}

int fclose(FILE *stream) {
  int fd = lnp64_file_fd(stream);
  if (fd > 2 && close(fd) < 0) {
    stream->error = 1;
    return EOF;
  }
  if (stream != stdin && stream != stdout && stream != stderr) {
    for (size_t i = 0; i < sizeof(lnp64_head_files) / sizeof(lnp64_head_files[0]);
         i++) {
      if (stream == &lnp64_head_files[i]) {
        lnp64_head_file_used[i] = 0;
        break;
      }
    }
  }
  return 0;
}

int fputc(int ch, FILE *stream) {
  unsigned char byte = (unsigned char)ch;
  int fd = lnp64_file_fd(stream);
  if (fd < 0 || write(fd, &byte, 1) != 1) {
    if (stream)
      stream->error = 1;
    return EOF;
  }
  return ch;
}

int putchar(int ch) { return fputc(ch, stdout); }

int fputs(const char *s, FILE *stream) {
  while (*s) {
    if (fputc((unsigned char)*s, stream) == EOF)
      return EOF;
    s++;
  }
  return 0;
}

size_t fwrite(const void *ptr, size_t size, size_t count, FILE *stream) {
  size_t bytes = size * count;
  int fd = lnp64_file_fd(stream);
  if (size == 0 || count == 0)
    return count;
  if (fd < 0)
    return 0;
  ssize_t written = write(fd, ptr, bytes);
  if (written <= 0) {
    stream->error = 1;
    return 0;
  }
  return (size_t)written / size;
}

int fgetc(FILE *stream) {
  unsigned char ch;
  int fd = lnp64_file_fd(stream);
  if (fd < 0) {
    if (stream)
      stream->error = 1;
    return EOF;
  }
  ssize_t got = read(fd, &ch, 1);
  if (got == 1)
    return ch;
  if (got == 0)
    stream->eof = 1;
  else
    stream->error = 1;
  return EOF;
}

int getc(FILE *stream) { return fgetc(stream); }

int getchar(void) { return fgetc(stdin); }

int fgetrune(Rune *r, FILE *stream) {
  int ch = fgetc(stream);
  if (ch == EOF)
    return 0;
  *r = ch < 0x80 ? ch : 0xfffd;
  return 1;
}

int efgetrune(Rune *r, FILE *stream, const char *name) {
  (void)name;
  return fgetrune(r, stream);
}

int fputrune(const Rune *r, FILE *stream) {
  char ch;
  if (!r)
    return EOF;
  ch = (*r >= 0 && *r < 0x80) ? (char)*r : '?';
  return fputc((unsigned char)ch, stream) == EOF ? EOF : 1;
}

int efputrune(const Rune *r, FILE *stream, const char *name) {
  (void)name;
  return fputrune(r, stream);
}

int ferror(FILE *stream) { return stream ? stream->error : 1; }

FILE *fopen(const char *path, const char *mode) {
  int flags = O_RDONLY;
  if (mode && mode[0] == 'w')
    flags = O_WRONLY | O_CREAT | O_TRUNC;
  int fd = open(path, flags, 0666);
  if (fd < 0)
    return 0;
  for (size_t i = 0; i < sizeof(lnp64_head_files) / sizeof(lnp64_head_files[0]);
       i++) {
    if (!lnp64_head_file_used[i]) {
      lnp64_head_file_used[i] = 1;
      lnp64_head_files[i].fd = fd;
      lnp64_head_files[i].eof = 0;
      lnp64_head_files[i].error = 0;
      return &lnp64_head_files[i];
    }
  }
  close(fd);
  return 0;
}

ssize_t getline(char **lineptr, size_t *n, FILE *stream) {
  size_t used = 0;
  if (!lineptr || !n || !stream)
    return -1;
  if (!*lineptr || *n == 0) {
    *n = 128;
    *lineptr = malloc(*n);
    if (!*lineptr) {
      stream->error = 1;
      *n = 0;
      return -1;
    }
  }
  while (1) {
    int ch = fgetc(stream);
    if (ch == EOF)
      break;
    if (used + 1 >= *n) {
      size_t next = *n * 2;
      if (next <= *n) {
        stream->error = 1;
        return -1;
      }
      char *grown = realloc(*lineptr, next);
      if (!grown) {
        stream->error = 1;
        return -1;
      }
      *lineptr = grown;
      *n = next;
    }
    (*lineptr)[used++] = (char)ch;
    if (ch == '\n')
      break;
  }
  if (used == 0)
    return -1;
  (*lineptr)[used] = 0;
  return (ssize_t)used;
}

int fshut(FILE *stream, const char *name) {
  (void)name;
  return fclose(stream);
}

static void lnp64_vprint(FILE *stream, const char *format, va_list ap) {
  while (*format) {
    if (*format != '%') {
      fputc((unsigned char)*format++, stream);
      continue;
    }
    format++;
    if (*format == '%') {
      fputc('%', stream);
      format++;
      continue;
    }
    if (*format == 's') {
      const char *s = va_arg(ap, const char *);
      fputs(s ? s : "(null)", stream);
      format++;
      continue;
    }
    while (*format >= '0' && *format <= '9')
      format++;
    if (*format == 'l' && format[1] == 'd') {
      long value = va_arg(ap, long);
      unsigned long magnitude;
      char buf[32];
      int pos = 0;
      if (value < 0) {
        fputc('-', stream);
        magnitude = (unsigned long)(-(value + 1)) + 1;
      } else {
        magnitude = (unsigned long)value;
      }
      do {
        buf[pos++] = (char)('0' + (magnitude % 10));
        magnitude /= 10;
      } while (magnitude);
      while (pos)
        fputc(buf[--pos], stream);
      format += 2;
      continue;
    }
    if (*format == 'z' && format[1] == 'u') {
      size_t value = va_arg(ap, size_t);
      char buf[32];
      int pos = 0;
      do {
        buf[pos++] = (char)('0' + (value % 10));
        value /= 10;
      } while (value);
      while (pos)
        fputc(buf[--pos], stream);
      format += 2;
      continue;
    }
    if (*format == 'u') {
      unsigned int value = va_arg(ap, unsigned int);
      char buf[16];
      int pos = 0;
      do {
        buf[pos++] = (char)('0' + (value % 10));
        value /= 10;
      } while (value);
      while (pos)
        fputc(buf[--pos], stream);
      format++;
      continue;
    }
    if (*format == 'o') {
      unsigned int value = va_arg(ap, unsigned int);
      char buf[16];
      int pos = 0;
      do {
        buf[pos++] = (char)('0' + (value & 7));
        value >>= 3;
      } while (value);
      while (pos)
        fputc(buf[--pos], stream);
      format++;
      continue;
    }
    fputc('%', stream);
  }
}

int printf(const char *format, ...) {
  va_list ap;
  va_start(ap, format);
  lnp64_vprint(stdout, format, ap);
  va_end(ap);
  return 0;
}

void xvprintf(const char *format, va_list ap) { lnp64_vprint(stderr, format, ap); }

static void lnp64_sbuf_put(char *str, size_t size, size_t *pos, char ch) {
  if (*pos + 1 < size)
    str[*pos] = ch;
  (*pos)++;
}

int vsnprintf(char *str, size_t size, const char *format, va_list ap) {
  size_t pos = 0;
  while (*format) {
    if (*format != '%') {
      lnp64_sbuf_put(str, size, &pos, *format++);
      continue;
    }
    format++;
    if (*format == '%') {
      lnp64_sbuf_put(str, size, &pos, *format++);
      continue;
    }
    if (*format == 's') {
      const char *s = va_arg(ap, const char *);
      if (!s)
        s = "(null)";
      while (*s)
        lnp64_sbuf_put(str, size, &pos, *s++);
      format++;
      continue;
    }
    lnp64_sbuf_put(str, size, &pos, '%');
    if (*format)
      lnp64_sbuf_put(str, size, &pos, *format++);
  }
  if (size) {
    size_t end = pos < size ? pos : size - 1;
    str[end] = 0;
  }
  return (int)pos;
}

int snprintf(char *str, size_t size, const char *format, ...) {
  va_list ap;
  int ret;
  va_start(ap, format);
  ret = vsnprintf(str, size, format, ap);
  va_end(ap);
  return ret;
}

int vfprintf(FILE *stream, const char *format, va_list ap) {
  lnp64_vprint(stream, format, ap);
  return 0;
}

int fprintf(FILE *stream, const char *format, ...) {
  va_list ap;
  va_start(ap, format);
  lnp64_vprint(stream, format, ap);
  va_end(ap);
  return 0;
}

void weprintf(const char *format, ...) {
  va_list ap;
  va_start(ap, format);
  lnp64_vprint(stderr, format, ap);
  va_end(ap);
}

void eprintf(const char *format, ...) {
  va_list ap;
  va_start(ap, format);
  lnp64_vprint(stderr, format, ap);
  va_end(ap);
  exit(1);
}

void enprintf(int status, const char *format, ...) {
  va_list ap;
  va_start(ap, format);
  lnp64_vprint(stderr, format, ap);
  va_end(ap);
  exit(status);
}

size_t estrlcpy(char *dst, const char *src, size_t size) {
  size_t len = strlen(src);
  if (size) {
    size_t copy = len < size - 1 ? len : size - 1;
    memcpy(dst, src, copy);
    dst[copy] = 0;
  }
  if (len >= size)
    eprintf("strlcpy:");
  return len;
}

size_t estrlcat(char *dst, const char *src, size_t size) {
  size_t used = strlen(dst);
  size_t len = used + strlen(src);
  if (used < size) {
    size_t room = size - used - 1;
    size_t copy = strlen(src) < room ? strlen(src) : room;
    memcpy(dst + used, src, copy);
    dst[used + copy] = 0;
  }
  if (len >= size)
    eprintf("strlcat:");
  return len;
}

struct dirent *readdir(DIR *dirp) {
  unsigned long status;
  memset(&lnp64_head_dirent, 0, sizeof(lnp64_head_dirent));
  __asm__ volatile("readdir_fd_dyn %1, %2\n\tmov %0, r1"
                   : "=r"(status)
                   : "r"((long)dirp), "r"(lnp64_head_dirent.d_name)
                   : "memory");
  if (status == 1)
    return &lnp64_head_dirent;
  return 0;
}

int fullrune(const char *s, size_t n) {
  (void)s;
  return n > 0;
}

size_t unescape(char *s) {
  char *dst = s;
  char *src = s;
  while (*src) {
    if (*src == '\\' && src[1]) {
      src++;
      if (*src == 'n')
        *dst++ = '\n';
      else if (*src == 't')
        *dst++ = '\t';
      else
        *dst++ = *src;
      src++;
      continue;
    }
    *dst++ = *src++;
  }
  *dst = 0;
  return (size_t)(dst - s);
}

void *xmemmem(const void *haystack, size_t haystacklen, const void *needle,
              size_t needlelen) {
  const unsigned char *h = haystack;
  const unsigned char *n = needle;
  if (needlelen == 0)
    return (void *)h;
  if (haystacklen < needlelen)
    return 0;
  for (size_t i = 0; i <= haystacklen - needlelen; i++) {
    if (!memcmp(h + i, n, needlelen))
      return (void *)(h + i);
  }
  return 0;
}

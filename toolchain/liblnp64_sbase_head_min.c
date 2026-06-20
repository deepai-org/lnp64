#include <fcntl.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

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

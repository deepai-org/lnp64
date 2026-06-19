#include <fcntl.h>
#include <stdarg.h>
#include <stddef.h>
#include <unistd.h>

typedef struct __lnp64_file FILE;

struct __lnp64_file {
  unsigned char *buffer;
  size_t size;
  size_t pos;
  int eof;
  int error;
  int fd;
  int kind;
  int has_unget;
  unsigned char unget;
};

enum {
  LNP64_STREAM_FREE = 0,
  LNP64_STREAM_MEMORY = 1,
  LNP64_STREAM_FD = 2,
};

static FILE lnp64_stdin_file = {.fd = 0, .kind = LNP64_STREAM_FD};
static FILE lnp64_stdout_file = {.fd = 1, .kind = LNP64_STREAM_FD};
static FILE lnp64_stderr_file = {.fd = 2, .kind = LNP64_STREAM_FD};

FILE *stdin = &lnp64_stdin_file;
FILE *stdout = &lnp64_stdout_file;
FILE *stderr = &lnp64_stderr_file;

static FILE lnp64_streams[8];
static unsigned long lnp64_tmpfile_counter;

static FILE *lnp64_alloc_stream(void) {
  size_t i;
  for (i = 0; i < sizeof lnp64_streams / sizeof lnp64_streams[0]; i = i + 1) {
    if (lnp64_streams[i].kind == LNP64_STREAM_FREE) {
      lnp64_streams[i].buffer = 0;
      lnp64_streams[i].size = 0;
      lnp64_streams[i].pos = 0;
      lnp64_streams[i].eof = 0;
      lnp64_streams[i].error = 0;
      lnp64_streams[i].fd = -1;
      lnp64_streams[i].has_unget = 0;
      lnp64_streams[i].unget = 0;
      return &lnp64_streams[i];
    }
  }
  return 0;
}

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

FILE *fmemopen(void *buf, unsigned long size, const char *mode) {
  (void)mode;
  if (!buf)
    return 0;
  FILE *stream = lnp64_alloc_stream();
  if (!stream)
    return 0;
  stream->kind = LNP64_STREAM_MEMORY;
  stream->buffer = (unsigned char *)buf;
  stream->size = size;
  stream->pos = 0;
  return stream;
}

int fileno(FILE *stream) {
  if (!stream || stream->kind != LNP64_STREAM_FD)
    return -1;
  return stream->fd;
}

int fflush(FILE *stream) {
  (void)stream;
  return 0;
}

int fclose(FILE *stream) {
  int rc = 0;
  if (!stream)
    return -1;
  if (stream->kind == LNP64_STREAM_FD && stream->fd > 2)
    rc = close(stream->fd);
  stream->has_unget = 0;
  stream->eof = 0;
  stream->error = 0;
  if (stream != stdin && stream != stdout && stream != stderr)
    stream->kind = LNP64_STREAM_FREE;
  return rc;
}

static long lnp64_stream_tell(FILE *stream) {
  if (!stream)
    return -1;
  return (long)stream->pos;
}

int fgetc(FILE *stream) {
  unsigned char ch;
  if (!stream) {
    return -1;
  }
  if (stream->has_unget) {
    stream->has_unget = 0;
    stream->pos = stream->pos + 1;
    return stream->unget;
  }
  if (stream->kind == LNP64_STREAM_MEMORY) {
    if (stream->pos >= stream->size) {
      stream->eof = 1;
      return -1;
    }
    ch = stream->buffer[stream->pos];
    stream->pos = stream->pos + 1;
    if (stream->pos >= stream->size)
      stream->eof = 1;
    return ch;
  }
  if (stream->kind == LNP64_STREAM_FD) {
    ssize_t got = read(stream->fd, &ch, 1);
    if (got == 1) {
      stream->pos = stream->pos + 1;
      return ch;
    }
    if (got == 0)
      stream->eof = 1;
    else
      stream->error = 1;
  }
  return -1;
}

int getc(FILE *stream) { return fgetc(stream); }

int ungetc(int ch, FILE *stream) {
  if (!stream || ch < 0)
    return -1;
  stream->has_unget = 1;
  stream->unget = (unsigned char)ch;
  if (stream->pos != 0)
    stream->pos = stream->pos - 1;
  stream->eof = 0;
  return (unsigned char)ch;
}

static int lnp64_stream_peek(FILE *stream) {
  if (stream && stream->has_unget)
    return stream->unget;
  int ch = fgetc(stream);
  if (ch < 0)
    return ch;
  if (stream->kind == LNP64_STREAM_FD) {
    if (lseek(stream->fd, -1, SEEK_CUR) >= 0 && stream->pos != 0)
      stream->pos = stream->pos - 1;
    return ch;
  }
  ungetc(ch, stream);
  return ch;
}

char *fgets(char *str, int count, FILE *stream) {
  int written = 0;
  if (!str || count <= 0 || !stream) {
    if (stream)
      stream->error = 1;
    return 0;
  }
  while (written + 1 < count) {
    int ch = fgetc(stream);
    if (ch < 0)
      break;
    str[written] = (char)ch;
    written = written + 1;
    if (ch == '\n')
      break;
  }
  if (written == 0)
    return 0;
  str[written] = 0;
  return str;
}

unsigned long fread(void *ptr, unsigned long size, unsigned long count,
                    FILE *stream) {
  unsigned char *out = (unsigned char *)ptr;
  unsigned long total = size * count;
  unsigned long i;
  if (size == 0 || count == 0)
    return 0;
  for (i = 0; i < total; i = i + 1) {
    int ch = fgetc(stream);
    if (ch < 0)
      break;
    out[i] = (unsigned char)ch;
  }
  return i / size;
}

int fputc(int ch, FILE *stream) {
  unsigned char byte = (unsigned char)ch;
  if (!stream)
    return -1;
  if (stream->kind == LNP64_STREAM_FD) {
    if (write(stream->fd, &byte, 1) == 1) {
      stream->pos = stream->pos + 1;
      return ch;
    }
    return -1;
  }
  if (stream->kind == LNP64_STREAM_MEMORY) {
    if (stream->pos >= stream->size) {
      stream->error = 1;
      return -1;
    }
    stream->buffer[stream->pos] = byte;
    stream->pos = stream->pos + 1;
    return ch;
  }
  return -1;
}

int fputs(const char *s, FILE *stream) {
  while (*s) {
    if (fputc(*s, stream) < 0)
      return -1;
    s = s + 1;
  }
  return 0;
}

unsigned long fwrite(const void *ptr, unsigned long size, unsigned long count,
                     FILE *stream) {
  const unsigned char *bytes = (const unsigned char *)ptr;
  unsigned long total = size * count;
  unsigned long i;
  if (size == 0 || count == 0)
    return count;
  if (stream && stream->kind == LNP64_STREAM_FD) {
    ssize_t written = write(stream->fd, ptr, total);
    if (written <= 0)
      return 0;
    stream->pos = stream->pos + (unsigned long)written;
    return (unsigned long)written / size;
  }
  for (i = 0; i < total; i = i + 1) {
    if (fputc(bytes[i], stream) < 0)
      break;
  }
  return i / size;
}

int vfprintf(FILE *stream, const char *format, va_list ap) {
  char buf[256];
  int len = vsnprintf(buf, sizeof buf, format, ap);
  unsigned long written_len;
  if (len < 0)
    return len;
  written_len = (unsigned long)len;
  if (written_len >= sizeof buf)
    written_len = sizeof buf - 1;
  if (fwrite(buf, 1, written_len, stream) != written_len)
    return -1;
  return len;
}

int fprintf(FILE *stream, const char *format, ...) {
  va_list ap;
  int ret;
  va_start(ap, format);
  ret = vfprintf(stream, format, ap);
  va_end(ap);
  return ret;
}

int printf(const char *format, ...) {
  va_list ap;
  int ret;
  va_start(ap, format);
  ret = vfprintf(stdout, format, ap);
  va_end(ap);
  return ret;
}

int fseek(FILE *stream, long offset, int whence) {
  long pos;
  if (!stream)
    return -1;
  stream->has_unget = 0;
  if (stream->kind == LNP64_STREAM_MEMORY) {
    long base = 0;
    if (whence == SEEK_CUR)
      base = (long)stream->pos;
    else if (whence == SEEK_END)
      base = (long)stream->size;
    else if (whence != SEEK_SET)
      return -1;
    pos = base + offset;
    if (pos < 0 || (unsigned long)pos > stream->size)
      return -1;
    stream->pos = (unsigned long)pos;
    stream->eof = 0;
    return 0;
  }
  if (stream->kind != LNP64_STREAM_FD)
    return -1;
  pos = lseek(stream->fd, offset, whence);
  if (pos < 0)
    return -1;
  stream->pos = (unsigned long)pos;
  stream->eof = 0;
  return 0;
}

long ftell(FILE *stream) { return lnp64_stream_tell(stream); }

int fscanf(FILE *stream, const char *format, ...) {
  const char *set;
  char *out;
  int matched = 0;
  va_list ap;
  if (!stream || !format || format[0] != '%' || format[1] != '[')
    return -1;
  set = format + 2;
  va_start(ap, format);
  out = va_arg(ap, char *);
  va_end(ap);
  while (*set && *set != ']') {
    int ch = lnp64_stream_peek(stream);
    if (ch < 0)
      break;
    if ((unsigned char)ch != (unsigned char)*set) {
      set = set + 1;
      continue;
    }
    do {
      ch = fgetc(stream);
      out[matched] = (char)ch;
      matched = matched + 1;
      ch = lnp64_stream_peek(stream);
    } while (ch >= 0 && (unsigned char)ch == (unsigned char)*set);
    break;
  }
  if (matched != 0)
    out[matched] = 0;
  return matched == 0 ? 0 : 1;
}

FILE *fopen(const char *path, const char *mode) {
  int flags = O_RDONLY;
  if (mode && mode[0] == 'w')
    flags = O_RDWR | O_CREAT | O_TRUNC;
  else if (mode && mode[0] == 'a')
    flags = O_RDWR | O_CREAT | O_APPEND;
  int fd = open(path, flags);
  if (fd < 0)
    return 0;
  FILE *stream = lnp64_alloc_stream();
  if (!stream) {
    close(fd);
    return 0;
  }
  stream->kind = LNP64_STREAM_FD;
  stream->fd = fd;
  stream->pos = 0;
  stream->eof = 0;
  stream->error = 0;
  stream->has_unget = 0;
  return stream;
}

FILE *tmpfile(void) {
  char path[64];
  int fd;
  FILE *stream;
  lnp64_tmpfile_counter = lnp64_tmpfile_counter + 1;
  snprintf(path, sizeof path, "/tmp/lnp64_tmpfile_%u",
           (unsigned)lnp64_tmpfile_counter);
  fd = open(path, O_RDWR | O_CREAT | O_TRUNC);
  if (fd < 0)
    return 0;
  stream = lnp64_alloc_stream();
  if (!stream) {
    close(fd);
    return 0;
  }
  stream->kind = LNP64_STREAM_FD;
  stream->fd = fd;
  stream->pos = 0;
  stream->eof = 0;
  stream->error = 0;
  stream->has_unget = 0;
  return stream;
}

int putchar(int ch) { return fputc(ch, stdout); }

int puts(const char *s) {
  if (fputs(s, stdout) < 0)
    return -1;
  return putchar('\n') < 0 ? -1 : 0;
}

int getchar(void) { return fgetc(stdin); }

int feof(FILE *stream) {
  return stream ? stream->eof : 0;
}

int ferror(FILE *stream) {
  return stream ? stream->error : 1;
}

void clearerr(FILE *stream) {
  if (!stream)
    return;
  stream->eof = 0;
  stream->error = 0;
}

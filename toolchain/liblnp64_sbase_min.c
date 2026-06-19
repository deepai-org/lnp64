#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

struct __lnp64_file {
  int fd;
};

static struct __lnp64_file lnp64_stdin_file = {0};
static struct __lnp64_file lnp64_stdout_file = {1};
static struct __lnp64_file lnp64_stderr_file = {2};

FILE *stdin = &lnp64_stdin_file;
FILE *stdout = &lnp64_stdout_file;
FILE *stderr = &lnp64_stderr_file;

char *argv0;

static int lnp64_file_fd(FILE *stream) {
  if (stream == stderr)
    return 2;
  if (stream == stdin)
    return 0;
  return 1;
}

int fileno(FILE *stream) { return lnp64_file_fd(stream); }

int fflush(FILE *stream) {
  (void)stream;
  return 0;
}

int fclose(FILE *stream) { return fflush(stream); }

int fputc(int ch, FILE *stream) {
  unsigned char byte = (unsigned char)ch;
  return write(lnp64_file_fd(stream), &byte, 1) == 1 ? ch : EOF;
}

int putchar(int ch) { return fputc(ch, stdout); }

int fputs(const char *s, FILE *stream) {
  size_t len = strlen(s);
  if (len == 0)
    return 0;
  return write(lnp64_file_fd(stream), s, len) == (ssize_t)len ? 0 : EOF;
}

int puts(const char *s) {
  if (fputs(s, stdout) == EOF)
    return EOF;
  return putchar('\n') == EOF ? EOF : 0;
}

size_t fwrite(const void *ptr, size_t size, size_t count, FILE *stream) {
  size_t bytes = size * count;
  if (bytes == 0)
    return count;
  ssize_t written = write(lnp64_file_fd(stream), ptr, bytes);
  if (written <= 0)
    return 0;
  return (size_t)written / size;
}

int fshut(FILE *stream, const char *name) {
  (void)name;
  return fflush(stream);
}

void efshut(FILE *stream, const char *name) {
  if (fshut(stream, name) != 0)
    exit(1);
}

void enfshut(int status, FILE *stream, const char *name) {
  if (fshut(stream, name) != 0)
    exit(status);
}

void putword(FILE *stream, const char *word) {
  static int need_space;
  if (need_space)
    fputc(' ', stream);
  fputs(word, stream);
  need_space = 1;
}

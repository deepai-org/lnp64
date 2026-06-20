#include <dirent.h>
#include <errno.h>
#include <fnmatch.h>
#include <grp.h>
#include <pwd.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "../third_party/sbase/util.h"

static struct dirent lnp64_find_dirent;

static int lnp64_match_classic(const char *pattern, const char *string) {
  while (*pattern) {
    if (*pattern == '*') {
      while (*pattern == '*')
        pattern++;
      if (!*pattern)
        return 1;
      while (*string) {
        if (lnp64_match_classic(pattern, string))
          return 1;
        string++;
      }
      return 0;
    }
    if (*pattern == '?') {
      if (!*string)
        return 0;
      pattern++;
      string++;
      continue;
    }
    if (*pattern != *string)
      return 0;
    pattern++;
    string++;
  }
  return *string == 0;
}

int fnmatch(const char *pattern, const char *string, int flags) {
  (void)flags;
  if (!pattern || !string)
    return FNM_NOMATCH;
  return lnp64_match_classic(pattern, string) ? 0 : FNM_NOMATCH;
}

void *emalloc(size_t size) {
  void *ptr = malloc(size);
  if (!ptr)
    eprintf("malloc:");
  return ptr;
}

void *erealloc(void *ptr, size_t size) {
  void *next = realloc(ptr, size);
  if (!next && size)
    eprintf("realloc:");
  return next;
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
  if (!next && count && size)
    eprintf("reallocarray:");
  return next;
}

char *estrdup(const char *s) {
  size_t len = strlen(s) + 1;
  char *copy = emalloc(len);
  memcpy(copy, s, len);
  return copy;
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
    size_t remaining = size - used;
    size_t copy = len - used < remaining - 1 ? len - used : remaining - 1;
    memcpy(dst + used, src, copy);
    dst[used + copy] = 0;
  }
  if (len >= size)
    eprintf("strlcat:");
  return len;
}

int fprintf(FILE *stream, const char *format, ...) {
  (void)stream;
  (void)format;
  return 0;
}

int fgetc(FILE *stream) {
  (void)stream;
  return EOF;
}

int feof(FILE *stream) {
  (void)stream;
  return 1;
}

void clearerr(FILE *stream) { (void)stream; }

struct dirent *readdir(DIR *dirp) {
  unsigned long status;
  memset(&lnp64_find_dirent, 0, sizeof(lnp64_find_dirent));
  __asm__ volatile("readdir_fd_dyn %1, %2\n\tmov %0, r1"
                   : "=r"(status)
                   : "r"((long)dirp), "r"(lnp64_find_dirent.d_name)
                   : "memory");
  if (status == 1)
    return &lnp64_find_dirent;
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

long sysconf(int name) {
  if (name == _SC_ARG_MAX)
    return _POSIX_ARG_MAX;
  errno = EINVAL;
  return -1;
}

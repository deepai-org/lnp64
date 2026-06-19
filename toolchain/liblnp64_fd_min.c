#include "lnp64_intrinsics.h"

typedef unsigned long size_t;

enum { LNP64_AT_FDCWD = -100 };
enum {
  LNP64_FDR_TOKEN_MARKER = 1UL << 62,
  LNP64_FDR_TOKEN_INDEX_MASK = 0xffUL,
  LNP64_O_RDWR = 0x0002,
  LNP64_O_CREAT = 0x0040,
  LNP64_O_EXCL = 0x0080,
  LNP64_O_TRUNC = 0x0200,
};

static unsigned long lnp64_mkstemp_counter;

long openat(int dirfd, const char *path, int flags) {
  lnp64_word_t result =
      __lnp_openat((lnp64_cap_t)(long)dirfd, (lnp64_word_t)path,
                   (lnp64_word_t)flags, 0);
  if ((long)result < 0)
    return (long)result;
  if (result & LNP64_FDR_TOKEN_MARKER)
    return (long)(result & LNP64_FDR_TOKEN_INDEX_MASK);
  return (long)result;
}

long open(const char *path, int flags) {
  return openat(LNP64_AT_FDCWD, path, flags);
}

long read(long fd, void *buf, size_t len) {
  return (long)__lnp_pull((lnp64_cap_t)fd, (lnp64_word_t)buf, len);
}

long write(long fd, const void *buf, size_t len) {
  return (long)__lnp_push((lnp64_cap_t)fd, (lnp64_word_t)buf, len);
}

long lseek(int fd, long offset, int whence) {
  unsigned long result;
  __asm__ volatile("fd_seek_dyn %1, %2, %3\n\tmov %0, r1"
                   : "=r"(result)
                   : "r"((long)fd), "r"(offset), "r"((long)whence)
                   : "memory");
  return (long)result;
}

int unlinkat(int dirfd, const char *path, int flags) {
  unsigned long result;
  __asm__ volatile("unlink_path_at %1, %2, %3\n\tmov %0, r1"
                   : "=r"(result)
                   : "r"((long)dirfd), "r"(path), "r"((long)flags)
                   : "memory");
  return (long)result < 0 ? -1 : 0;
}

int unlink(const char *path) {
  return unlinkat(LNP64_AT_FDCWD, path, 0);
}

int mkstemp(char *template) {
  unsigned long len = 0;
  unsigned long value;
  char *suffix;
  static const char digits[] = "0123456789abcdefghijklmnopqrstuvwxyz";
  if (!template)
    return -1;
  while (template[len])
    len = len + 1;
  if (len < 6)
    return -1;
  suffix = template + len - 6;
  if (suffix[0] != 'X' || suffix[1] != 'X' || suffix[2] != 'X' ||
      suffix[3] != 'X' || suffix[4] != 'X' || suffix[5] != 'X')
    return -1;

  lnp64_mkstemp_counter = lnp64_mkstemp_counter + 1;
  value = lnp64_mkstemp_counter;
  for (int i = 5; i >= 0; i = i - 1) {
    suffix[i] = digits[value % 36];
    value = value / 36;
  }
  return (int)open(template, LNP64_O_RDWR | LNP64_O_CREAT | LNP64_O_EXCL |
                                LNP64_O_TRUNC);
}

int close(int fd) {
  return __lnp_cap_revoke((lnp64_cap_t)(long)fd, 0) >= 0 ? 0 : -1;
}

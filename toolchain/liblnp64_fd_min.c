#include <lnp64/intrinsics.h>

#include <fcntl.h>
#include <unistd.h>

enum {
  LNP64_FDR_TOKEN_MARKER = 1UL << 62,
  LNP64_FDR_TOKEN_INDEX_MASK = 0xffUL,
};

static unsigned long lnp64_mkstemp_counter;

int openat(int dirfd, const char *path, int flags, ...) {
  lnp64_word_t result =
      __lnp_openat((lnp64_cap_t)(long)dirfd, (lnp64_word_t)path,
                   (lnp64_word_t)flags, 0);
  if ((long)result < 0)
    return (int)(long)result;
  if (result & LNP64_FDR_TOKEN_MARKER)
    return (int)(result & LNP64_FDR_TOKEN_INDEX_MASK);
  return (int)result;
}

int open(const char *path, int flags, ...) {
  return openat(AT_FDCWD, path, flags);
}

int creat(const char *path, mode_t mode) {
  (void)mode;
  return open(path, O_WRONLY | O_CREAT | O_TRUNC);
}

ssize_t read(int fd, void *buf, size_t len) {
  return (long)__lnp_pull((lnp64_cap_t)fd, (lnp64_word_t)buf, len);
}

ssize_t write(int fd, const void *buf, size_t len) {
  return (long)__lnp_push((lnp64_cap_t)fd, (lnp64_word_t)buf, len);
}

off_t lseek(int fd, off_t offset, int whence) {
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
  return unlinkat(AT_FDCWD, path, 0);
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
  return open(template, O_RDWR | O_CREAT | O_EXCL | O_TRUNC);
}

int close(int fd) {
  return __lnp_cap_revoke((lnp64_cap_t)(long)fd, 0) >= 0 ? 0 : -1;
}

#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <sys/stat.h>
#include <unistd.h>

extern int lnp64_errno_store(int value);

static struct stat lnp64_access_stat;

static int lnp64_errno_load(void) {
  unsigned long value;
  __asm__ volatile("errno_get %0" : "=r"(value) : : "memory");
  return (int)value;
}

static int lnp64_complete_status(long status) {
  if (status < 0) {
    lnp64_errno_store(lnp64_errno_load());
    return -1;
  }
  return (int)status;
}

static int lnp64_stat_path_at(int dirfd, const char *path, struct stat *st,
                              int flags) {
  unsigned long status;
  unsigned long native_flags = (flags & AT_SYMLINK_NOFOLLOW) ? 1UL : 0UL;
  __asm__ volatile("stat_path_at %1, %2, %3, %4\n\tmov %0, r1"
                   : "=r"(status)
                   : "r"(st), "r"((long)dirfd), "r"(path), "r"(native_flags)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int fstatat(int dirfd, const char *path, struct stat *st, int flags) {
  return lnp64_stat_path_at(dirfd, path, st, flags);
}

int stat(const char *path, struct stat *st) {
  return lnp64_stat_path_at(AT_FDCWD, path, st, 0);
}

int access(const char *path, int mode) {
  if (mode & ~(R_OK | W_OK | X_OK)) {
    lnp64_errno_store(EINVAL);
    return -1;
  }
  return lnp64_stat_path_at(AT_FDCWD, path, &lnp64_access_stat, 0);
}

int lstat(const char *path, struct stat *st) {
  return lnp64_stat_path_at(AT_FDCWD, path, st, AT_SYMLINK_NOFOLLOW);
}

int fstat(int fd, struct stat *st) {
  unsigned long status;
  __asm__ volatile("stat_fd_dyn %1, %2\n\tmov %0, r1"
                   : "=r"(status)
                   : "r"(st), "r"((long)fd)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int utimensat(int dirfd, const char *path, const struct timespec times[2],
              int flags) {
  unsigned long status;
  unsigned long native_flags = (flags & AT_SYMLINK_NOFOLLOW) ? 1UL : 0UL;
  __asm__ volatile("utime_path_at %1, %2, %3, %4\n\tmov %0, r1"
                   : "=r"(status)
                   : "r"((long)dirfd), "r"(path), "r"(times),
                     "r"(native_flags)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int futimens(int fd, const struct timespec times[2]) {
  unsigned long status;
  __asm__ volatile("utime_fd_dyn %1, %2\n\tmov %0, r1"
                   : "=r"(status)
                   : "r"((long)fd), "r"(times)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int fcntl(int fd, int cmd, ...) {
  unsigned long status;
  unsigned long arg = 0;
  va_list ap;
  if (cmd == F_DUPFD || cmd == F_SETFD || cmd == F_SETFL) {
    va_start(ap, cmd);
    arg = (unsigned long)va_arg(ap, long);
    va_end(ap);
  } else if (cmd == F_GETLK || cmd == F_SETLK || cmd == F_SETLKW) {
    va_start(ap, cmd);
    arg = (unsigned long)va_arg(ap, struct flock *);
    va_end(ap);
  }
  __asm__ volatile("fcntl_fd_dyn %1, %2, %3\n\tmov %0, r1"
                   : "=r"(status)
                   : "r"((long)fd), "r"((long)cmd), "r"(arg)
                   : "memory");
  return lnp64_complete_status((long)status);
}

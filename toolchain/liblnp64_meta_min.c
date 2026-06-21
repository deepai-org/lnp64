#include <lnp64/intrinsics.h>
#include <errno.h>
#include <dirent.h>
#include <fcntl.h>
#include <stdarg.h>
#include <sys/stat.h>
#include <unistd.h>

extern int lnp64_errno_store(int value);

static struct stat lnp64_access_stat;
static mode_t lnp64_umask_value = 022;

enum {
  LNP64_FDR_TOKEN_MARKER = 1UL << 62,
  LNP64_FDR_TOKEN_INDEX_MASK = 0xffUL,
};

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

static long lnp64_complete_long(long status) {
  if (status < 0) {
    lnp64_errno_store(lnp64_errno_load());
    return -1;
  }
  return status;
}

static unsigned long lnp64_at_flags(int flags) {
  return (flags & AT_SYMLINK_NOFOLLOW) ? 1UL : 0UL;
}

static int lnp64_stat_path_at(int dirfd, const char *path, struct stat *st,
                              int flags) {
  unsigned long status;
  unsigned long native_flags = lnp64_at_flags(flags);
  __asm__ volatile("stat_path_at %1, %2, %3, %4\n\tmov %0, r2"
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

int faccessat(int dirfd, const char *path, int mode, int flags) {
  if (mode & ~(R_OK | W_OK | X_OK)) {
    lnp64_errno_store(EINVAL);
    return -1;
  }
  return lnp64_stat_path_at(dirfd, path, &lnp64_access_stat, flags);
}

int lstat(const char *path, struct stat *st) {
  return lnp64_stat_path_at(AT_FDCWD, path, st, AT_SYMLINK_NOFOLLOW);
}

int fstat(int fd, struct stat *st) {
  unsigned long status;
  __asm__ volatile("stat_fd_dyn %1, %2\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"(st), "r"((long)fd)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int utimensat(int dirfd, const char *path, const struct timespec times[2],
              int flags) {
  unsigned long status;
  unsigned long native_flags = lnp64_at_flags(flags);
  __asm__ volatile("utime_path_at %1, %2, %3, %4\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)dirfd), "r"(path), "r"(times),
                     "r"(native_flags)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int futimens(int fd, const struct timespec times[2]) {
  unsigned long status;
  __asm__ volatile("utime_fd_dyn %1, %2\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)fd), "r"(times)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int chdir(const char *path) {
  unsigned long status;
  __asm__ volatile("chdir_path %1\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"(path)
                   : "memory");
  return lnp64_complete_status((long)status);
}

char *getcwd(char *buf, size_t size) {
  unsigned long status;
  if (!buf) {
    lnp64_errno_store(EINVAL);
    return 0;
  }
  __asm__ volatile("getcwd_path %1, %2\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"(buf), "r"((long)size)
                   : "memory");
  return lnp64_complete_long((long)status) < 0 ? 0 : buf;
}

DIR *opendir(const char *name) {
  unsigned long result;
  __asm__ volatile("open_dir_dyn %0, %1, %2"
                   : "=r"(result)
                   : "r"(name), "r"(0UL)
                   : "memory");
  if ((long)result < 0)
    return 0;
  if (result & LNP64_FDR_TOKEN_MARKER)
    return (DIR *)(long)(result & LNP64_FDR_TOKEN_INDEX_MASK);
  return (DIR *)(long)result;
}

int closedir(DIR *dirp) {
  if (!dirp) {
    lnp64_errno_store(EBADF);
    return -1;
  }
  return close((int)(long)dirp);
}

static struct dirent lnp64_readdir_ent;

struct dirent *readdir(DIR *dirp) {
  if (!dirp) return 0;
  /* Each read from the directory fd returns one null-terminated name */
  unsigned long n = (unsigned long)__lnp_pull(
      (lnp64_cap_t)(long)dirp,
      (lnp64_word_t)lnp64_readdir_ent.d_name,
      sizeof(lnp64_readdir_ent.d_name) - 1);
  if ((long)n <= 0) return 0;
  lnp64_readdir_ent.d_name[n] = '\0';
  lnp64_readdir_ent.d_ino = 0;
  lnp64_readdir_ent.d_type = 0;
  return &lnp64_readdir_ent;
}

int mkdirat(int dirfd, const char *path, mode_t mode) {
  unsigned long status;
  __asm__ volatile("mkdir_path_at %1, %2, %3\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)dirfd), "r"(path), "r"((long)mode)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int mkdir(const char *path, mode_t mode) {
  return mkdirat(AT_FDCWD, path, mode);
}

int mknod(const char *path, mode_t mode, dev_t dev) {
  (void)path;
  (void)mode;
  (void)dev;
  lnp64_errno_store(ENOSYS);
  return -1;
}

mode_t umask(mode_t mask) {
  mode_t old = lnp64_umask_value;
  lnp64_umask_value = mask & 0777;
  return old;
}

int renameat(int olddirfd, const char *oldpath, int newdirfd,
             const char *newpath) {
  unsigned long status;
  __asm__ volatile("rename_path_at %1, %2, %3, %4\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)olddirfd), "r"(oldpath), "r"((long)newdirfd),
                     "r"(newpath)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int rename(const char *oldpath, const char *newpath) {
  return renameat(AT_FDCWD, oldpath, AT_FDCWD, newpath);
}

int linkat(int olddirfd, const char *oldpath, int newdirfd,
           const char *newpath, int flags) {
  unsigned long status;
  __asm__ volatile("link_path_at %1, %2, %3, %4, %5\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)olddirfd), "r"(oldpath), "r"((long)newdirfd),
                     "r"(newpath), "r"((long)flags)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int symlinkat(const char *target, int newdirfd, const char *linkpath) {
  unsigned long status;
  __asm__ volatile("symlink_path_at %1, %2, %3\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"(target), "r"((long)newdirfd), "r"(linkpath)
                   : "memory");
  return lnp64_complete_status((long)status);
}

int symlink(const char *target, const char *linkpath) {
  return symlinkat(target, AT_FDCWD, linkpath);
}

ssize_t readlinkat(int dirfd, const char *path, char *buf, size_t bufsiz) {
  unsigned long status;
  __asm__ volatile("readlink_path_at %1, %2, %3, %4\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)dirfd), "r"(path), "r"(buf), "r"((long)bufsiz)
                   : "memory");
  return (ssize_t)lnp64_complete_long((long)status);
}

ssize_t readlink(const char *path, char *buf, size_t bufsiz) {
  return readlinkat(AT_FDCWD, path, buf, bufsiz);
}

int fchmodat(int dirfd, const char *path, mode_t mode, int flags) {
  unsigned long status;
  __asm__ volatile("chmod_path_at %1, %2, %3, %4\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)dirfd), "r"(path), "r"((long)mode),
                     "r"(lnp64_at_flags(flags))
                   : "memory");
  return lnp64_complete_status((long)status);
}

int chmod(const char *path, mode_t mode) {
  return fchmodat(AT_FDCWD, path, mode, 0);
}

int fchownat(int dirfd, const char *path, uid_t owner, gid_t group, int flags) {
  unsigned long status;
  long native_owner = owner == (uid_t)-1 ? -1L : (long)owner;
  long native_group = group == (gid_t)-1 ? -1L : (long)group;
  __asm__ volatile("chown_path_at %1, %2, %3, %4, %5\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)dirfd), "r"(path), "r"(native_owner),
                     "r"(native_group), "r"(lnp64_at_flags(flags))
                   : "memory");
  return lnp64_complete_status((long)status);
}

int chown(const char *path, uid_t owner, gid_t group) {
  return fchownat(AT_FDCWD, path, owner, group, 0);
}

int lchown(const char *path, uid_t owner, gid_t group) {
  return fchownat(AT_FDCWD, path, owner, group, AT_SYMLINK_NOFOLLOW);
}

int rmdir(const char *path) {
  return unlinkat(AT_FDCWD, path, AT_REMOVEDIR);
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
  __asm__ volatile("fcntl_fd_dyn %1, %2, %3\n\tmov %0, r2"
                   : "=r"(status)
                   : "r"((long)fd), "r"((long)cmd), "r"(arg)
                   : "memory");
  return lnp64_complete_status((long)status);
}

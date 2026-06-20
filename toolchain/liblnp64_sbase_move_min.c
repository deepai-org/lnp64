#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "../third_party/sbase/fs.h"
#include "../third_party/sbase/util.h"

int cp_aflag;
int cp_fflag;
int cp_iflag;
int cp_pflag;
int cp_rflag;
int cp_vflag;
int cp_status;
int cp_follow;

static void lnp64_cp_error(const char *op, const char *path) {
  weprintf("%s %s:", op, path);
  cp_status = 1;
}

int cp(const char *src, const char *dst, int depth) {
  char buf[256];
  int in_fd;
  int out_fd;
  int stat_flags = 0;
  ssize_t got;
  struct stat st;
  (void)depth;

  if (cp_follow == 'P')
    stat_flags = AT_SYMLINK_NOFOLLOW;
  if (fstatat(AT_FDCWD, src, &st, stat_flags) < 0) {
    lnp64_cp_error("stat", src);
    return 0;
  }
  if (S_ISDIR(st.st_mode)) {
    errno = EISDIR;
    lnp64_cp_error("copy", src);
    return 0;
  }

  in_fd = open(src, O_RDONLY);
  if (in_fd < 0) {
    lnp64_cp_error("open", src);
    return 0;
  }
  out_fd = open(dst, O_WRONLY | O_CREAT | O_TRUNC, st.st_mode & 0777);
  if (out_fd < 0) {
    lnp64_cp_error("open", dst);
    close(in_fd);
    return 0;
  }

  while ((got = read(in_fd, buf, sizeof buf)) > 0) {
    char *p = buf;
    ssize_t left = got;
    while (left > 0) {
      ssize_t put = write(out_fd, p, (size_t)left);
      if (put <= 0) {
        lnp64_cp_error("write", dst);
        close(in_fd);
        close(out_fd);
        return 0;
      }
      p += put;
      left -= put;
    }
  }
  if (got < 0)
    lnp64_cp_error("read", src);
  if (close(in_fd) < 0)
    lnp64_cp_error("close", src);
  if (close(out_fd) < 0)
    lnp64_cp_error("close", dst);
  if ((cp_aflag || cp_pflag) && chmod(dst, st.st_mode & 0777) < 0)
    lnp64_cp_error("chmod", dst);
  return 0;
}

void fnck(const char *a, const char *b,
          int (*fn)(const char *, const char *, int), int depth) {
  struct stat sta;
  struct stat stb;

  if (!stat(a, &sta) && !stat(b, &stb) && sta.st_dev == stb.st_dev &&
      sta.st_ino == stb.st_ino) {
    weprintf("%s -> %s: same file\n", a, b);
    return;
  }

  if (fn(a, b, depth) < 0)
    eprintf("%s -> %s:", a, b);
}

void enmasse(int argc, char *argv[],
             int (*fn)(const char *, const char *, int)) {
  struct stat st;

  if (argc == 2 && !(stat(argv[1], &st) == 0 && S_ISDIR(st.st_mode))) {
    fnck(argv[0], argv[1], fn, 0);
    return;
  }

  eprintf("sbase mv directory mode is not implemented in this minimal gate\n");
}

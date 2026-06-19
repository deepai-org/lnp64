#include <fcntl.h>
#include <unistd.h>

static int write_all(int fd, const char *buf, unsigned long len) {
  unsigned long off = 0;
  while (off < len) {
    ssize_t n = write(fd, buf + off, len - off);
    if (n <= 0)
      return -1;
    off = off + (unsigned long)n;
  }
  return 0;
}

static int write_cstr(int fd, const char *s) {
  unsigned long len = 0;
  while (s[len])
    len = len + 1;
  return write_all(fd, s, len);
}

static int cat_file(const char *path) {
  char buf[128];
  int fd = open(path, O_RDONLY);
  if (fd < 0)
    return -1;

  for (;;) {
    ssize_t n = read(fd, buf, sizeof(buf));
    if (n < 0)
      return -1;
    if (n == 0)
      break;
    if (write_all(STDOUT_FILENO, buf, (unsigned long)n) != 0)
      return -1;
  }

  return close(fd);
}

int main(int argc, char **argv) {
  const char *root_name = argc > 1 ? argv[1] : "/";

  write_all(STDOUT_FILENO, "lnp64 clang init: boot\n", 23);
  write_all(STDOUT_FILENO, "lnp64 clang init: root ", 23);
  write_cstr(STDOUT_FILENO, root_name);
  write_all(STDOUT_FILENO, "\n", 1);

  return cat_file("etc/motd") == 0 ? 0 : 1;
}

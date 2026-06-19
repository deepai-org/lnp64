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

int main(int argc, char **argv) {
  const char *path = argc > 1 ? argv[1] : "etc/motd";
  char buf[128];
  int fd = open(path, O_RDONLY);
  if (fd < 0) {
    write_all(STDERR_FILENO, "ucat: open failed\n", 18);
    return 1;
  }

  for (;;) {
    ssize_t n = read(fd, buf, sizeof(buf));
    if (n < 0)
      return 2;
    if (n == 0)
      break;
    if (write_all(STDOUT_FILENO, buf, (unsigned long)n) != 0)
      return 3;
  }

  return close(fd) == 0 ? 0 : 4;
}

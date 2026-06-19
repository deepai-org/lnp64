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

int main(void) {
  write_all(STDOUT_FILENO, "lnpsh clang: scripted console\n", 30);
  write_all(STDOUT_FILENO, "$ pwd\n/\n", 8);

  write_all(STDOUT_FILENO, "$ cat etc/motd\n", 15);
  if (cat_file("etc/motd") != 0)
    return 1;

  write_all(STDOUT_FILENO, "$ devs\n", 7);
  if (cat_file("dev/devices") != 0)
    return 2;

  write_all(STDOUT_FILENO, "lnpsh clang: halt\n", 18);
  return 0;
}

#include <unistd.h>

static int write_cstr(int fd, const char *text) {
  int n = 0;
  while (text[n] != 0)
    n = n + 1;
  write(fd, text, (unsigned long)n);
  return n;
}

int main(int argc, char **argv) {
  const char *root = argc > 1 ? argv[1] : "/";

  write(1, "lnp64-netbsd-personality: supervisor boot\n", 42);
  write(1, "lnp64-netbsd-personality: root ", 31);
  write_cstr(1, root);
  write(1, "\n", 1);

  execl("/bin/netbsd_sh.elf", "netbsd_sh", root, 0);
  write(2, "netbsd clang init: exec failed\n", 31);
  return 1;
}

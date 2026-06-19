#include <errno.h>
#include <sys/mman.h>
#include <unistd.h>

int main(void) {
  unsigned char *p =
      mmap(0, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (p == MAP_FAILED)
    return 1;

  p[0] = 42;
  if (p[0] != 42)
    return 2;
  if (mprotect(p, 4096, PROT_READ) != 0)
    return 3;
  if (p[0] != 42)
    return 4;
  if (munmap(p, 4096) != 0)
    return 5;

  errno = 0;
  if (mprotect(p, 4096, PROT_READ) != -1)
    return 6;
  if (errno != ENOMEM)
    return 7;

  write(1, "mmap_test ok\n", 13);
  return 0;
}

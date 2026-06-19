#include <errno.h>
#include <fcntl.h>
#include <unistd.h>

#include "test.h"

#define TEST(c)                                                               \
  do {                                                                        \
    errno = 0;                                                                \
    if (!(c))                                                                 \
      t_error("%s failed (errno = %d)\n", #c, errno);                         \
  } while (0)

int main(void) {
  int fd = open("Cargo.toml", O_RDONLY);
  if (fd < 0) {
    t_error("open Cargo.toml failed (errno = %d)\n", errno);
    return t_status;
  }

  TEST(fcntl(fd, F_GETFD) == 0);
  TEST(fcntl(fd, F_SETFD, 0L) == 0);
  TEST(fcntl(fd, F_GETFL) == 0);
  TEST(fcntl(fd, F_SETFL, 0L) == 0);

  errno = 0;
  if (fcntl(fd, 999, 0) != -1 || errno != EINVAL)
    t_error("unsupported fcntl command did not report EINVAL\n");

  close(fd);
  return t_status;
}

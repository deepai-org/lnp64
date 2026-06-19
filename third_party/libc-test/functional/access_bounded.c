#include <errno.h>
#include <unistd.h>

#include "test.h"

#define TEST(c)                                                               \
  do {                                                                        \
    errno = 0;                                                                \
    if (!(c))                                                                 \
      t_error("%s failed (errno = %d)\n", #c, errno);                         \
  } while (0)

int main(void) {
  TEST(access("Cargo.toml", F_OK) == 0);
  TEST(access("Cargo.toml", R_OK) == 0);
  TEST(access("Cargo.toml", W_OK) == 0);
  TEST(access("Cargo.toml", X_OK) == 0);

  errno = 0;
  if (access("target/lnp64-definitely-missing-access-fixture", F_OK) != -1 ||
      errno != ENOENT)
    t_error("missing path did not report ENOENT through access\n");

  errno = 0;
  if (access("Cargo.toml", 0x40) != -1 || errno != EINVAL)
    t_error("invalid access mode did not report EINVAL\n");

  return t_status;
}

#include <lnp64/intrinsics.h>

#include <unistd.h>

static volatile lnp64_word_t child_marker;

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

static int child_entry(lnp64_word_t arg) {
  child_marker = arg + 1;
  write_all(STDOUT_FILENO, "userland spawn: child\n",
            sizeof("userland spawn: child\n") - 1);
  __lnp_exit(0);
  return 0;
}

int main(void) {
  write_all(STDOUT_FILENO, "userland spawn: parent\n",
            sizeof("userland spawn: parent\n") - 1);

  lnp64_word_t tid = __lnp_spawn_entry((lnp64_word_t)child_entry, 41);
  if (tid == (lnp64_word_t)-1)
    return 1;
  if (__lnp_thread_join(tid, 0) != 0)
    return 2;
  if (child_marker != 42)
    return 3;

  write_all(STDOUT_FILENO, "userland spawn: joined\n",
            sizeof("userland spawn: joined\n") - 1);
  return 0;
}

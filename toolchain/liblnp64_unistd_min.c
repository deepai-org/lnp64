#include <unistd.h>
#include <lnp64/intrinsics.h>
#include <errno.h>

int dup(int oldfd) { (void)oldfd; errno = ENOSYS; return -1; }
int dup2(int oldfd, int newfd) { (void)oldfd; (void)newfd; errno = ENOSYS; return -1; }
int isatty(int fd) { (void)fd; return 0; }
int gethostname(char *name, size_t len) {
  static const char h[] = "lnp64";
  size_t i;
  for (i = 0; i < sizeof(h) && i < len - 1; i++) name[i] = h[i];
  if (len > 0) name[i] = '\0';
  return 0;
}
int sethostname(const char *name, size_t len) { (void)name; (void)len; return 0; }

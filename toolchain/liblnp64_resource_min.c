#include <sys/resource.h>
#include <errno.h>
#include <string.h>

int getrlimit(int resource, struct rlimit *rlp) {
  (void)resource;
  if (!rlp) { errno = EINVAL; return -1; }
  rlp->rlim_cur = RLIM_INFINITY;
  rlp->rlim_max = RLIM_INFINITY;
  return 0;
}

int setrlimit(int resource, const struct rlimit *rlp) {
  (void)resource; (void)rlp;
  return 0;
}

int getrusage(int who, struct rusage *usage) {
  (void)who;
  if (!usage) { errno = EINVAL; return -1; }
  memset(usage, 0, sizeof(*usage));
  return 0;
}

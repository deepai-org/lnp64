#ifndef LNP64_SYS_SELECT_H
#define LNP64_SYS_SELECT_H

#include <sys/time.h>

#define FD_SETSIZE 64

typedef struct {
  unsigned long bits[16];
} fd_set;

static inline void FD_ZERO(fd_set *s) {
  for (int i = 0; i < 16; i++) s->bits[i] = 0;
}
static inline void FD_SET(int fd, fd_set *s) {
  if (fd >= 0 && fd < FD_SETSIZE)
    s->bits[fd / 64] |= (1UL << (fd % 64));
}
static inline void FD_CLR(int fd, fd_set *s) {
  if (fd >= 0 && fd < FD_SETSIZE)
    s->bits[fd / 64] &= ~(1UL << (fd % 64));
}
static inline int FD_ISSET(int fd, const fd_set *s) {
  if (fd < 0 || fd >= FD_SETSIZE) return 0;
  return (s->bits[fd / 64] >> (fd % 64)) & 1;
}

int select(int nfds, fd_set *readfds, fd_set *writefds, fd_set *exceptfds,
           struct timeval *timeout);

#endif

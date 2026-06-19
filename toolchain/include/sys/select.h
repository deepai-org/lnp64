#ifndef LNP64_SYS_SELECT_H
#define LNP64_SYS_SELECT_H

#define FD_SETSIZE 64

typedef struct {
  unsigned long bits[16];
} fd_set;

struct timeval {
  long tv_sec;
  long tv_usec;
};

int select(int nfds, fd_set *readfds, fd_set *writefds, fd_set *exceptfds,
           struct timeval *timeout);

#endif

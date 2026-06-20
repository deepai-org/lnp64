#ifndef LNP64_POLL_H
#define LNP64_POLL_H

typedef unsigned long nfds_t;

struct pollfd {
  int fd;
  short events;
  short revents;
};

#define POLLIN  0x01
#define POLLPRI 0x02
#define POLLOUT 0x04
#define POLLERR 0x08
#define POLLHUP 0x10
#define POLLNVAL 0x20

int poll(struct pollfd *fds, nfds_t nfds, int timeout);

#endif

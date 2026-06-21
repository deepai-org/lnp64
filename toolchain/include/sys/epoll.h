#ifndef LNP64_SYS_EPOLL_H
#define LNP64_SYS_EPOLL_H

#define EPOLLIN 0x01
#define EPOLLOUT 0x04
#define EPOLLERR 0x08
#define EPOLLHUP 0x10

#define EPOLL_CTL_ADD 1
#define EPOLL_CTL_DEL 2
#define EPOLL_CTL_MOD 3

struct epoll_event {
  unsigned int events;
  unsigned long data;
};

int epoll_create(int size);
int epoll_create1(int flags);
int epoll_ctl(int epfd, int op, int fd, struct epoll_event *event);
int epoll_wait(int epfd, struct epoll_event *events, int maxevents,
               int timeout);

#define EPOLL_CLOEXEC 0x80000

#endif

#ifndef LNP64_SYS_SOCKET_H
#define LNP64_SYS_SOCKET_H

#include <stddef.h>

typedef unsigned long socklen_t;

#define AF_INET 2

#define SOCK_STREAM 1

#define SOL_SOCKET 1
#define SO_REUSEADDR 2
#define SO_BROADCAST 3
#define SO_ERROR 4
#define SO_SNDBUF 5
#define SO_RCVBUF 6
#define SO_KEEPALIVE 9

#define MSG_NOSIGNAL 0x4000

int socket(int domain, int type, int protocol);
int bind(int fd, const void *addr, socklen_t len);
int listen(int fd, int backlog);
int connect(int fd, const void *addr, socklen_t len);
int accept(int fd, void *addr, socklen_t *len);
int getsockname(int fd, void *addr, socklen_t *len);
int getsockopt(int fd, int level, int optname, void *optval,
               socklen_t *optlen);
int setsockopt(int fd, int level, int optname, const void *optval,
               socklen_t optlen);
long send(int fd, const void *buf, size_t len, int flags);
long recv(int fd, void *buf, size_t len, int flags);

#endif

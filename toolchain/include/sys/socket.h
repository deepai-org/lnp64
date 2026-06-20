#ifndef LNP64_SYS_SOCKET_H
#define LNP64_SYS_SOCKET_H

#include <stddef.h>

typedef unsigned long socklen_t;

#define AF_INET    2
#define AF_INET6   10
#define AF_UNIX    1
#define AF_UNSPEC  0
#define AF_LOCAL   AF_UNIX

struct sockaddr {
  unsigned char sa_len;
  unsigned char sa_family;
  char sa_data[14];
};

#define SOCK_STREAM    1
#define SOCK_DGRAM     2
#define SOCK_NONBLOCK  0x800
#define SOCK_CLOEXEC   0x80000

#define SOL_SOCKET   1
#define SO_REUSEADDR 2
#define SO_BROADCAST 3
#define SO_ERROR     4
#define SO_SNDBUF    7
#define SO_RCVBUF    8
#define SO_KEEPALIVE 9
#define SO_SNDTIMEO  20
#define SO_RCVTIMEO  21
#define SO_LINGER    13
#define SO_REUSEPORT 15
#define SO_TYPE      3

#define MSG_NOSIGNAL 0x4000
#define MSG_DONTWAIT 0x40
#define MSG_PEEK     0x02
#define MSG_WAITALL  0x100

#define IPPROTO_TCP  6
#define IPPROTO_UDP  17
#define IPPROTO_IP   0
#define IPPROTO_IPV6 41

struct linger {
  int l_onoff;
  int l_linger;
};

int getpeername(int fd, void *addr, socklen_t *len);
long sendto(int fd, const void *buf, size_t len, int flags,
            const void *dest_addr, socklen_t addrlen);
long recvfrom(int fd, void *buf, size_t len, int flags,
              void *src_addr, socklen_t *addrlen);
int shutdown(int fd, int how);

#define SHUT_RD   0
#define SHUT_WR   1
#define SHUT_RDWR 2

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

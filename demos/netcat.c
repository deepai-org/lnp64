#include <lnp64/intrinsics.h>

typedef unsigned long size_t;
typedef unsigned long nfds_t;
typedef unsigned long socklen_t;

enum {
  AF_INET = 2,
  SOCK_STREAM = 1,
  POLLIN = 0x01,
  MSG_NOSIGNAL = 0x4000
};

struct pollfd {
  int fd;
  short events;
  short revents;
};

void *malloc(size_t size);
int socket(int domain, int type, int protocol);
int bind(int fd, const void *addr, socklen_t len);
int listen(int fd, int backlog);
int accept(int fd, void *addr, socklen_t *len);
int poll(struct pollfd *fds, nfds_t nfds, int timeout);
long recv(int fd, void *buf, size_t len, int flags);
long send(int fd, const void *buf, size_t len, int flags);
long write(long fd, const void *buf, size_t len);

int main(void) {
  struct pollfd p[1];
  int server;
  int client;
  char *buf;
  long n;

  server = socket(AF_INET, SOCK_STREAM, 0);
  if (server < 0)
    return 1;
  if (bind(server, "127.0.0.1:41065", 0) != 0)
    return 2;
  if (listen(server, 1) != 0)
    return 3;
  write(1, "netcat ready\n", 13);

  buf = malloc(128);
  if (!buf)
    return 4;
  p[0].fd = server;
  p[0].events = POLLIN;
  while (poll(p, 1, 0) == 0) {
  }

  client = accept(server, 0, 0);
  if (client < 0)
    return 5;
  n = recv(client, buf, 128, 0);
  if (n < 0)
    return 6;
  write(1, buf, (size_t)n);
  if (send(client, buf, (size_t)n, MSG_NOSIGNAL) != n)
    return 7;
  __lnp_exit(0);
  return 0;
}

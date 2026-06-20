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
long open(const char *path, int flags);
long read(long fd, void *buf, size_t len);
long write(long fd, const void *buf, size_t len);
int socket(int domain, int type, int protocol);
int bind(int fd, const void *addr, socklen_t len);
int listen(int fd, int backlog);
int accept(int fd, void *addr, socklen_t *len);
int connect(int fd, const void *addr, socklen_t len);
int poll(struct pollfd *fds, nfds_t nfds, int timeout);
long recv(int fd, void *buf, size_t len, int flags);
long send(int fd, const void *buf, size_t len, int flags);

static int handler(int client) {
  char *buf = malloc(256);
  char *filebuf = malloc(64);
  if (!buf || !filebuf)
    return 1;

  recv(client, buf, 256, 0);
  long fd = open("demos/index.html", 0);
  if (fd == -1)
    return 2;

  long n = read(fd, filebuf, 64);
  if (n < 0)
    return 3;
  send(client, "HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n", 39,
       MSG_NOSIGNAL);
  send(client, filebuf, (size_t)n, MSG_NOSIGNAL);
  return 0;
}

static int self_test_arg(int argc, char **argv) {
  return argc > 1 && argv[1][0] == '-' && argv[1][1] == '-' &&
         argv[1][2] == 's';
}

int main(int argc, char **argv) {
  struct pollfd p[1];
  int self_test = self_test_arg(argc, argv);
  int probe = -1;
  int server = socket(AF_INET, SOCK_STREAM, 0);
  if (server < 0)
    return 1;
  if (bind(server, "127.0.0.1:41066", 0) != 0)
    return 2;
  if (listen(server, 1) != 0)
    return 3;
  write(1, "httpd ready\n", 12);

  if (self_test) {
    probe = socket(AF_INET, SOCK_STREAM, 0);
    if (probe < 0)
      return 6;
    if (connect(probe, "127.0.0.1:41066", 0) != 0)
      return 7;
    if (send(probe, "GET / HTTP/1.0\r\n\r\n", 18, MSG_NOSIGNAL) != 18)
      return 8;
  }

  p[0].fd = server;
  p[0].events = POLLIN;
  while (poll(p, 1, 0) == 0) {
  }

  int client = accept(server, 0, 0);
  if (client < 0)
    return 4;
  if (handler(client) != 0)
    return 5;
  if (self_test) {
    char *reply = malloc(96);
    if (!reply)
      return 9;
    if (recv(probe, reply, 96, 0) <= 0)
      return 10;
    write(1, "httpd self-test ok\n", 19);
  }
  return 0;
}

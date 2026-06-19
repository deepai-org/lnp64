#include "lnp64_intrinsics.h"

#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <unistd.h>

typedef unsigned long size_t;
typedef unsigned long nfds_t;
typedef unsigned long socklen_t;

struct pollfd {
  int fd;
  short events;
  short revents;
};

enum {
  AF_INET = 2,
  SOCK_STREAM = 1,
  SOL_SOCKET = 1,
  SO_REUSEADDR = 2,
  SO_ERROR = 4,
  MSG_NOSIGNAL = 0x4000,
  POLLIN = 0x01,
  MAP_PRIVATE = 0x02,
  MAP_ANONYMOUS = 0x20,
};

void *mmap(void *addr, size_t len, int prot, int flags, int fd, long offset);
int mprotect(void *addr, size_t len, int prot);
int munmap(void *addr, size_t len);
int poll(struct pollfd *fds, nfds_t nfds, int timeout);
int socket(int domain, int type, int protocol);
int bind(int fd, const void *addr, socklen_t len);
int listen(int fd, int backlog);
int connect(int fd, const void *addr, socklen_t len);
int accept(int fd, void *addr, socklen_t *len);
int getsockname(int fd, void *addr, socklen_t *len);
int setsockopt(int fd, int level, int optname, const void *optval,
               socklen_t optlen);
long send(int fd, const void *buf, size_t len, int flags);
long recv(int fd, void *buf, size_t len, int flags);

static volatile unsigned long thread_marker;

static int write_all(int fd, const char *buf, unsigned long len) {
  unsigned long off = 0;
  while (off < len) {
    ssize_t n = write(fd, buf + off, len - off);
    if (n <= 0)
      return -1;
    off = off + (unsigned long)n;
  }
  return 0;
}

static void *thread_entry(void *arg) {
  thread_marker = (unsigned long)arg + 1;
  return (void *)thread_marker;
}

static int check_root_image(void) {
  char buf[8];
  int fd = open("demos/netbsd_rumpfs.img", O_RDONLY);
  if (fd < 0)
    return 1;
  if (read(fd, buf, sizeof(buf)) != (ssize_t)sizeof(buf))
    return 2;
  if (close(fd) != 0)
    return 3;
  if (buf[0] != 'R' || buf[1] != 'U' || buf[2] != 'M' || buf[3] != 'P')
    return 4;
  if (buf[4] != 'F' || buf[5] != 'S')
    return 5;
  return 0;
}

static int check_mmap(void) {
  unsigned char *bytes =
      mmap(0, 4096, 3, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (bytes == (void *)~0UL)
    return 1;
  bytes[0] = 0x4e;
  bytes[1] = 0x42;
  if (bytes[0] != 0x4e || bytes[1] != 0x42)
    return 2;
  if (mprotect(bytes, 4096, 1) != 0)
    return 3;
  if (bytes[0] != 0x4e)
    return 4;
  return munmap(bytes, 4096) == 0 ? 0 : 5;
}

static int check_thread(void) {
  pthread_t thread;
  void *retval = 0;
  thread_marker = 0;
  if (pthread_create(&thread, 0, thread_entry, (void *)41) != 0)
    return 1;
  if (pthread_join(thread, &retval) != 0)
    return 2;
  if (thread_marker != 42)
    return 3;
  return (unsigned long)retval == 42 ? 0 : 4;
}

static int check_signal_compat(void) {
  if (signal(SIGTERM, SIG_IGN) != 0)
    return 1;
  return raise(SIGTERM) == 0 ? 0 : 2;
}

static int check_poll_compat(void) {
  struct pollfd p[1];
  p[0].fd = -1;
  p[0].events = POLLIN;
  p[0].revents = 7;
  if (poll(p, 1, 0) != 0)
    return 1;
  return p[0].revents == 0 ? 0 : 2;
}

static int check_socket_poll(void) {
  int server;
  int client;
  int accepted;
  int opt = 1;
  char addr[64];
  socklen_t addrlen = sizeof(addr);
  char buf[1];
  int i;
  long got;

  server = socket(AF_INET, SOCK_STREAM, 0);
  if (server < 0)
    return 1;
  if (setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) != 0)
    return 2;
  if (bind(server, "127.0.0.1:0", 0) != 0)
    return 3;
  if (listen(server, 1) != 0)
    return 4;
  if (getsockname(server, addr, &addrlen) != 0)
    return 5;
  client = socket(AF_INET, SOCK_STREAM, 0);
  if (client < 0)
    return 6;
  if (connect(client, addr, addrlen) != 0)
    return 7;
  accepted = -1;
  for (i = 0; i < 1000; i = i + 1) {
    accepted = accept(server, 0, 0);
    if (accepted >= 0)
      break;
  }
  if (accepted < 0)
    return 8;
  if (send(client, "n", 1, MSG_NOSIGNAL) != 1)
    return 9;
  got = -1;
  for (i = 0; i < 1000; i = i + 1) {
    got = recv(accepted, buf, 1, 0);
    if (got == 1)
      break;
  }
  if (got != 1)
    return 10;
  return buf[0] == 'n' ? 0 : 11;
}

int main(void) {
  int rc;
  static const char init_msg[] = "netbsd clang personality init\n";
  static const char shell_msg[] = "netbsd clang personality shell\n";
  static const char ok_msg[] = "netbsd clang personality smoke ok\n";
  if (write_all(STDOUT_FILENO, init_msg, sizeof(init_msg) - 1) != 0)
    return 1;
  if (write_all(STDOUT_FILENO, shell_msg, sizeof(shell_msg) - 1) != 0)
    return 2;
  rc = check_root_image();
  if (rc != 0)
    return 10 + rc;
  rc = check_mmap();
  if (rc != 0)
    return 20 + rc;
  rc = check_thread();
  if (rc != 0)
    return 30 + rc;
  rc = check_signal_compat();
  if (rc != 0)
    return 40 + rc;
  rc = check_poll_compat();
  if (rc != 0)
    return 45 + rc;
  rc = check_socket_poll();
  if (rc != 0)
    return 50 + rc;
  if (write_all(STDOUT_FILENO, ok_msg, sizeof(ok_msg) - 1) != 0)
    return 3;
  return 0;
}

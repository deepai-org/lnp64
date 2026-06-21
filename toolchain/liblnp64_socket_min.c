#include <lnp64/intrinsics.h>

#include <sys/socket.h>
#include <netinet/in.h>

enum {
  LNP64_OBJECT_OP_CREATE = 1,
  LNP64_OBJECT_OP_SOCKET_BIND = 2,
  LNP64_OBJECT_OP_SOCKET_LISTEN = 3,
  LNP64_OBJECT_OP_SOCKET_CONNECT = 4,
  LNP64_OBJECT_OP_SOCKET_ACCEPT = 5,
  LNP64_OBJECT_OP_SOCKET_GETSOCKNAME = 6,
  LNP64_OBJECT_OP_SOCKET_GETSOCKOPT = 7,
  LNP64_OBJECT_OP_SOCKET_SETSOCKOPT = 8,
  LNP64_OBJECT_KIND_ENDPOINT = 5,
  LNP64_OBJECT_PROFILE_TCP_STREAM = 2
};

int lnp64_errno_store(int value);

static int lnp64_errno_load(void) {
  unsigned long value;
  __asm__ volatile("errno_get %0" : "=r"(value) : : "memory");
  return (int)value;
}

static long lnp64_complete_status(long status) {
  if (status < 0) {
    lnp64_errno_store(lnp64_errno_load());
    return -1;
  }
  return status;
}

static int lnp64_socket_ctl(unsigned long op, int fd, unsigned long requested_fd,
                            unsigned long arg, unsigned long aux) {
  lnp64_word_t record[9];
  record[0] = op;
  record[1] = 0;
  record[2] = 0;
  record[3] = (lnp64_word_t)(unsigned int)fd;
  record[4] = requested_fd;
  record[5] = arg;
  record[6] = aux;
  record[7] = 0;
  record[8] = 0;
  return (int)lnp64_complete_status((long)__lnp_object_ctl((lnp64_word_t)record));
}

int socket(int domain, int type, int protocol) {
  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_OP_CREATE;
  record[1] = LNP64_OBJECT_KIND_ENDPOINT;
  record[2] = LNP64_OBJECT_PROFILE_TCP_STREAM;
  record[3] = 0;
  record[4] = 0;
  record[5] = (lnp64_word_t)(unsigned int)domain;
  record[6] = (lnp64_word_t)(unsigned int)type;
  record[7] = (lnp64_word_t)(unsigned int)protocol;
  record[8] = 0;
  return (int)lnp64_complete_status((long)__lnp_object_ctl((lnp64_word_t)record));
}

/* LNP64 emulator bind/connect expect a C string "a.b.c.d:port" */
static void lnp64_fmt_uint(char *buf, unsigned int v, int *pos) {
  if (v >= 100) { buf[(*pos)++] = '0' + (char)(v / 100); v %= 100; }
  if (v >= 10)  { buf[(*pos)++] = '0' + (char)(v / 10);  v %= 10; }
  buf[(*pos)++] = '0' + (char)v;
}

/* Convert struct sockaddr_in to "ip:port\0" in buf (needs >=24 bytes) */
static int lnp64_sockaddr_to_str(const void *addr_v, char *buf) {
  const struct sockaddr_in *sa = (const struct sockaddr_in *)addr_v;
  unsigned int ip = ntohl(sa->sin_addr.s_addr);
  unsigned int port = ntohs(sa->sin_port);
  int pos = 0;
  lnp64_fmt_uint(buf, (ip >> 24) & 0xff, &pos); buf[pos++] = '.';
  lnp64_fmt_uint(buf, (ip >> 16) & 0xff, &pos); buf[pos++] = '.';
  lnp64_fmt_uint(buf, (ip >>  8) & 0xff, &pos); buf[pos++] = '.';
  lnp64_fmt_uint(buf,  ip        & 0xff, &pos); buf[pos++] = ':';
  lnp64_fmt_uint(buf, port,              &pos);
  buf[pos] = '\0';
  return pos;
}

int bind(int fd, const void *addr, socklen_t len) {
  (void)len;
  return lnp64_socket_ctl(LNP64_OBJECT_OP_SOCKET_BIND, fd, 0,
                          (lnp64_word_t)addr, (lnp64_word_t)len);
}

int listen(int fd, int backlog) {
  return lnp64_socket_ctl(LNP64_OBJECT_OP_SOCKET_LISTEN, fd, 0,
                          (lnp64_word_t)(unsigned int)backlog, 0);
}

int connect(int fd, const void *addr, socklen_t len) {
  return lnp64_socket_ctl(LNP64_OBJECT_OP_SOCKET_CONNECT, fd, 0,
                          (lnp64_word_t)addr, (lnp64_word_t)len);
}

int accept(int fd, void *addr, socklen_t *len) {
  (void)addr;
  (void)len;
  return lnp64_socket_ctl(LNP64_OBJECT_OP_SOCKET_ACCEPT, fd, 0, 0, 0);
}

int getsockname(int fd, void *addr, socklen_t *len) {
  return lnp64_socket_ctl(LNP64_OBJECT_OP_SOCKET_GETSOCKNAME, fd, 0,
                          (lnp64_word_t)addr, (lnp64_word_t)len);
}

int getsockopt(int fd, int level, int optname, void *optval, socklen_t *optlen) {
  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_OP_SOCKET_GETSOCKOPT;
  record[1] = 0;
  record[2] = 0;
  record[3] = (lnp64_word_t)(unsigned int)fd;
  record[4] = 0;
  record[5] = (lnp64_word_t)(unsigned int)level;
  record[6] = (lnp64_word_t)(unsigned int)optname;
  record[7] = (lnp64_word_t)optval;
  record[8] = (lnp64_word_t)optlen;
  return (int)lnp64_complete_status((long)__lnp_object_ctl((lnp64_word_t)record));
}

int setsockopt(int fd, int level, int optname, const void *optval,
               socklen_t optlen) {
  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_OP_SOCKET_SETSOCKOPT;
  record[1] = 0;
  record[2] = 0;
  record[3] = (lnp64_word_t)(unsigned int)fd;
  record[4] = 0;
  record[5] = (lnp64_word_t)(unsigned int)level;
  record[6] = (lnp64_word_t)(unsigned int)optname;
  record[7] = (lnp64_word_t)optval;
  record[8] = (lnp64_word_t)optlen;
  return (int)lnp64_complete_status((long)__lnp_object_ctl((lnp64_word_t)record));
}

long send(int fd, const void *buf, size_t len, int flags) {
  (void)flags;
  return lnp64_complete_status((long)__lnp_push(
      (lnp64_cap_t)(unsigned long)fd, (lnp64_word_t)buf, len));
}

long recv(int fd, void *buf, size_t len, int flags) {
  (void)flags;
  return lnp64_complete_status((long)__lnp_pull(
      (lnp64_cap_t)(unsigned long)fd, (lnp64_word_t)buf, len));
}

#include <unistd.h>

int pipe(int pipefd[2]) {
  /* Use native LNP64 pipe: ObjectKind::Queue=2, ObjectProfile::Pipe=1 */
  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_OP_CREATE;
  record[1] = 2; /* ObjectKind::Queue */
  record[2] = 1; /* ObjectProfile::Pipe */
  record[3] = 0; /* desired read fd (0 = auto) */
  record[4] = 0; /* desired write fd (0 = auto) */
  record[5] = 0;
  record[6] = 0;
  record[7] = 0;
  record[8] = 0;
  long ret = (long)__lnp_object_ctl((lnp64_word_t)record);
  if (ret < 0) {
    lnp64_errno_store(lnp64_errno_load());
    return -1;
  }
  pipefd[0] = (int)(unsigned int)(unsigned long)record[3];
  pipefd[1] = (int)(unsigned int)(unsigned long)record[4];
  return 0;
}

int pipe2(int pipefd[2], int flags) {
  (void)flags;
  return pipe(pipefd);
}

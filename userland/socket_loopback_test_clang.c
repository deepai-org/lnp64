#include <errno.h>
#include <netinet/in.h>
#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
  struct pollfd p[1];
  int server;
  int client;
  int accepted;
  char addr[64];
  socklen_t addrlen;
  char buf;
  unsigned long opt;
  socklen_t optlen;

  server = socket(AF_INET, SOCK_STREAM, 0);
  if (server == -1)
    return 1;
  opt = 1;
  if (setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) != 0)
    return 2;
  opt = 123;
  optlen = sizeof(opt);
  if (getsockopt(server, SOL_SOCKET, SO_ERROR, &opt, &optlen) != 0)
    return 3;
  if (opt != 0)
    return 4;
  errno = 0;
  if (setsockopt(server, SOL_SOCKET, 999, &opt, sizeof(opt)) != -1)
    return 5;
  if (errno != EINVAL)
    return 6;
  if (bind(server, "127.0.0.1:0", 0) != 0)
    return 7;
  if (listen(server, 1) != 0)
    return 8;

  addrlen = sizeof(addr);
  if (getsockname(server, addr, &addrlen) != 0)
    return 9;
  client = socket(AF_INET, SOCK_STREAM, 0);
  if (client == -1)
    return 10;
  if (connect(client, addr, addrlen) != 0)
    return 11;
  p[0].fd = server;
  p[0].events = POLLIN;
  p[0].revents = 0;
  if (poll(p, 1, 0) != 1)
    return 12;
  accepted = accept(server, 0, 0);
  if (accepted == -1)
    return 13;
  if (send(client, "n", 1, MSG_NOSIGNAL) != 1)
    return 14;
  p[0].fd = accepted;
  p[0].events = POLLIN;
  p[0].revents = 0;
  if (poll(p, 1, 0) != 1)
    return 15;
  if (recv(accepted, &buf, 1, 0) != 1)
    return 16;
  if (buf != 'n')
    return 17;

  write(1, "socket_loopback_test ok\n", 24);
  return 0;
}

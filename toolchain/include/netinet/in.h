#ifndef LNP64_NETINET_IN_H
#define LNP64_NETINET_IN_H

#include <sys/socket.h>

#define IPPROTO_TCP 6
#define TCP_NODELAY 1

typedef unsigned int in_addr_t;
typedef unsigned short in_port_t;

struct in_addr {
  in_addr_t s_addr;
};

struct sockaddr_in {
  unsigned char sin_len;
  unsigned char sin_family;
  in_port_t sin_port;
  struct in_addr sin_addr;
  unsigned char sin_zero[8];
};

#endif

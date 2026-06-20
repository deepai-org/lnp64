#ifndef LNP64_NETINET_IN_H
#define LNP64_NETINET_IN_H

#include <sys/socket.h>
#include <stdint.h>

#define IPPROTO_TCP 6
#define TCP_NODELAY 1

typedef uint32_t in_addr_t;
typedef uint16_t in_port_t;

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

static inline uint16_t htons(uint16_t x) {
  return ((x & 0xff) << 8) | ((x >> 8) & 0xff);
}

static inline uint32_t htonl(uint32_t x) {
  return ((x & 0xff) << 24) | (((x >> 8) & 0xff) << 16) |
         (((x >> 16) & 0xff) << 8) | ((x >> 24) & 0xff);
}

static inline uint16_t ntohs(uint16_t x) {
  return htons(x);
}

static inline uint32_t ntohl(uint32_t x) {
  return htonl(x);
}

#endif

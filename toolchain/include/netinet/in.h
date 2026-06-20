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

struct in6_addr {
  unsigned char s6_addr[16];
};

struct sockaddr_in6 {
  unsigned char sin6_len;
  unsigned char sin6_family;
  in_port_t sin6_port;
  uint32_t sin6_flowinfo;
  struct in6_addr sin6_addr;
  uint32_t sin6_scope_id;
};

/* Generic storage large enough for any sockaddr */
struct sockaddr_storage {
  unsigned char ss_family;
  unsigned char __ss_pad[127];
};

#define IN6_IS_ADDR_UNSPECIFIED(a) 0
#define IN6_IS_ADDR_LOOPBACK(a)    0

#define INET_ADDRSTRLEN  16
#define INET6_ADDRSTRLEN 46

#define IPV6_V6ONLY      26
#define IPV6_JOIN_GROUP  20
#define IPV6_LEAVE_GROUP 21
#define IPV6_MULTICAST_IF 17
#define IPV6_MULTICAST_LOOP 19
#define IPV6_RECVPKTINFO 49
#define IPV6_PKTINFO     50
#define IP_MULTICAST_IF  32
#define IP_MULTICAST_LOOP 34
#define IP_ADD_MEMBERSHIP 35
#define IP_DROP_MEMBERSHIP 36
#define IP_TOS           1
#define IP_TTL           2
#define IP_HDRINCL       3
#define IPPROTO_ICMP     1
#define IPPROTO_ICMPV6   58

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

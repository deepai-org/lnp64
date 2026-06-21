#include <netdb.h>
#include <netinet/in.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

struct hostent *gethostbyname(const char *name) {
  (void)name;
  return 0;
}

static unsigned int lnp64_parse_ipv4(const char *s) {
  unsigned int a = 0, b = 0, c = 0, d = 0;
  int n = 0;
  while (*s && n < 4) {
    unsigned int v = 0;
    while (*s >= '0' && *s <= '9') { v = v * 10 + (*s++ - '0'); }
    if (n == 0) a = v; else if (n == 1) b = v;
    else if (n == 2) c = v; else d = v;
    n++;
    if (*s == '.') s++;
  }
  return (a << 24) | (b << 16) | (c << 8) | d;
}

static unsigned short lnp64_parse_port(const char *s) {
  unsigned short p = 0;
  while (*s >= '0' && *s <= '9') { p = (unsigned short)(p * 10 + (*s++ - '0')); }
  return p;
}

int getaddrinfo(const char *node, const char *service,
                const struct addrinfo *hints, struct addrinfo **res) {
  int family = hints ? hints->ai_family : AF_INET;
  int socktype = hints ? hints->ai_socktype : SOCK_STREAM;
  if (family != AF_INET && family != AF_UNSPEC) { errno = EAFNOSUPPORT; return EAI_FAMILY; }

  unsigned int addr4 = 0;
  if (!node) {
    addr4 = 0; /* INADDR_ANY */
  } else if (strcmp(node, "localhost") == 0 || strcmp(node, "127.0.0.1") == 0) {
    addr4 = 0x7f000001;
  } else {
    /* try dotted quad */
    const char *p = node;
    int dots = 0;
    while (*p) { if (*p == '.') dots++; p++; }
    if (dots == 3) {
      addr4 = lnp64_parse_ipv4(node);
    } else {
      return EAI_NONAME;
    }
  }

  unsigned short port = service ? lnp64_parse_port(service) : 0;

  struct sockaddr_in *sa = (struct sockaddr_in *)malloc(sizeof(struct sockaddr_in));
  if (!sa) return EAI_MEMORY;
  struct addrinfo *ai = (struct addrinfo *)malloc(sizeof(struct addrinfo));
  if (!ai) { free(sa); return EAI_MEMORY; }

  memset(sa, 0, sizeof(*sa));
  sa->sin_family = AF_INET;
  /* htons: port is stored big-endian */
  sa->sin_port = (unsigned short)((port >> 8) | (port << 8));
  sa->sin_addr.s_addr = ((addr4 >> 24) & 0xff)
    | (((addr4 >> 16) & 0xff) << 8)
    | (((addr4 >>  8) & 0xff) << 16)
    | ( (addr4        & 0xff) << 24);

  memset(ai, 0, sizeof(*ai));
  ai->ai_family = AF_INET;
  ai->ai_socktype = socktype;
  ai->ai_protocol = 0;
  ai->ai_addrlen = sizeof(struct sockaddr_in);
  ai->ai_addr = (struct sockaddr *)sa;
  ai->ai_canonname = 0;
  ai->ai_next = 0;

  *res = ai;
  return 0;
}

void freeaddrinfo(struct addrinfo *res) {
  while (res) {
    struct addrinfo *next = res->ai_next;
    if (res->ai_addr) free(res->ai_addr);
    free(res);
    res = next;
  }
}

const char *gai_strerror(int errcode) {
  switch (errcode) {
  case EAI_AGAIN:   return "name resolution temporarily failed";
  case EAI_FAIL:    return "name resolution failed";
  case EAI_FAMILY:  return "address family not supported";
  case EAI_MEMORY:  return "out of memory";
  case EAI_NONAME:  return "name not found";
  case EAI_SERVICE: return "service not found";
  default:          return "name resolution error";
  }
}

int getnameinfo(const struct sockaddr *addr, socklen_t addrlen,
                char *host, socklen_t hostlen,
                char *serv, socklen_t servlen, int flags) {
  (void)flags;
  if (addr && addr->sa_family == AF_INET && addrlen >= (socklen_t)sizeof(struct sockaddr_in)) {
    const struct sockaddr_in *sa = (const struct sockaddr_in *)addr;
    if (host && hostlen > 0) {
      unsigned int ip = ((unsigned char *)&sa->sin_addr.s_addr)[0] << 24 |
                        ((unsigned char *)&sa->sin_addr.s_addr)[1] << 16 |
                        ((unsigned char *)&sa->sin_addr.s_addr)[2] << 8  |
                        ((unsigned char *)&sa->sin_addr.s_addr)[3];
      /* format dotted quad */
      int pos = 0;
      unsigned int parts[4] = {(ip>>24)&0xff,(ip>>16)&0xff,(ip>>8)&0xff,ip&0xff};
      for (int i = 0; i < 4 && pos < (int)hostlen - 1; i++) {
        if (i) host[pos++] = '.';
        unsigned int v = parts[i];
        if (v >= 100) host[pos++] = '0' + (char)(v / 100);
        if (v >= 10)  host[pos++] = '0' + (char)((v / 10) % 10);
        host[pos++] = '0' + (char)(v % 10);
      }
      host[pos] = '\0';
    }
    if (serv && servlen > 0) {
      unsigned short port = (unsigned short)((sa->sin_port >> 8) | (sa->sin_port << 8));
      int pos = 0;
      if (port >= 10000) serv[pos++] = '0' + (char)(port / 10000);
      if (port >= 1000)  serv[pos++] = '0' + (char)((port / 1000) % 10);
      if (port >= 100)   serv[pos++] = '0' + (char)((port / 100) % 10);
      if (port >= 10)    serv[pos++] = '0' + (char)((port / 10) % 10);
      serv[pos++] = '0' + (char)(port % 10);
      serv[pos] = '\0';
    }
    return 0;
  }
  if (host && hostlen > 0) host[0] = '\0';
  if (serv && servlen > 0) serv[0] = '\0';
  return EAI_FAIL;
}

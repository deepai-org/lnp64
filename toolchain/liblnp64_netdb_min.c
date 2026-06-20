#include <netdb.h>
#include <string.h>
#include <errno.h>

struct hostent *gethostbyname(const char *name) {
  (void)name;
  return 0;
}

int getaddrinfo(const char *node, const char *service,
                const struct addrinfo *hints, struct addrinfo **res) {
  (void)node; (void)service; (void)hints; (void)res;
  return EAI_FAIL;
}

void freeaddrinfo(struct addrinfo *res) { (void)res; }

const char *gai_strerror(int errcode) {
  (void)errcode;
  return "name resolution not supported";
}

int getnameinfo(const struct sockaddr *addr, socklen_t addrlen,
                char *host, socklen_t hostlen,
                char *serv, socklen_t servlen, int flags) {
  (void)addr; (void)addrlen; (void)flags;
  if (host && hostlen > 0) host[0] = '\0';
  if (serv && servlen > 0) serv[0] = '\0';
  return EAI_FAIL;
}

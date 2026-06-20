#ifndef LNP64_SYS_UN_H
#define LNP64_SYS_UN_H

#include <sys/socket.h>

#define UNIX_PATH_MAX 108

struct sockaddr_un {
  unsigned short sun_family;
  char sun_path[UNIX_PATH_MAX];
};

#endif

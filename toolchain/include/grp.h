#ifndef LNP64_GRP_H
#define LNP64_GRP_H

#include <sys/types.h>

struct group {
  char *gr_name;
  gid_t gr_gid;
};

struct group *getgrgid(gid_t gid);
struct group *getgrnam(const char *name);

#endif

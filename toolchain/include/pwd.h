#ifndef LNP64_PWD_H
#define LNP64_PWD_H

#include <sys/types.h>

struct passwd {
  char *pw_name;
  uid_t pw_uid;
  gid_t pw_gid;
};

struct passwd *getpwnam(const char *name);
struct passwd *getpwuid(uid_t uid);

#endif

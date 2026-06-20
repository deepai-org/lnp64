#include <grp.h>
#include <pwd.h>

struct passwd *getpwnam(const char *name) {
  (void)name;
  return 0;
}

struct passwd *getpwuid(uid_t uid) {
  (void)uid;
  return 0;
}

struct group *getgrnam(const char *name) {
  (void)name;
  return 0;
}

struct group *getgrgid(gid_t gid) {
  (void)gid;
  return 0;
}

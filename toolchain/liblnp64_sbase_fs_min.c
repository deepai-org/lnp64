#include <errno.h>
#include <limits.h>
#include <stddef.h>
#include <string.h>
#include <sys/stat.h>

mode_t getumask(void) {
  mode_t mask = umask(0);
  umask(mask);
  return mask;
}

mode_t parsemode(const char *s, mode_t current, mode_t mask) {
  mode_t value = 0;
  if (!s || !*s)
    return current & ~mask;
  for (const char *p = s; *p; p++) {
    if (*p < '0' || *p > '7')
      return current & ~mask;
    value = (value << 3) | (mode_t)(*p - '0');
  }
  return value;
}

static int lnp64_mkdir_existing_dir(const char *path) {
  struct stat st;
  if (stat(path, &st) == 0 && S_ISDIR(st.st_mode))
    return 0;
  errno = EEXIST;
  return -1;
}

int mkdirp(const char *path, mode_t mode, mode_t parent_mode) {
  char buf[PATH_MAX];
  size_t len;

  if (!path || !*path) {
    errno = ENOENT;
    return -1;
  }

  len = strlen(path);
  if (len >= sizeof buf) {
    errno = ENAMETOOLONG;
    return -1;
  }
  memcpy(buf, path, len + 1);
  while (len > 1 && buf[len - 1] == '/')
    buf[--len] = 0;

  for (char *p = buf + 1; *p; p++) {
    if (*p != '/')
      continue;
    *p = 0;
    if (mkdir(buf, parent_mode) < 0) {
      if (errno != EEXIST)
        return -1;
      if (lnp64_mkdir_existing_dir(buf) < 0)
        return -1;
    }
    *p = '/';
  }

  if (mkdir(buf, mode) < 0) {
    if (errno == EEXIST)
      return lnp64_mkdir_existing_dir(buf);
    return -1;
  }
  return 0;
}

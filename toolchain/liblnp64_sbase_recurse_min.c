#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "../third_party/sbase/fs.h"
#include "../third_party/sbase/util.h"

int recurse_status;
int rm_status;

void recurse(int dirfd, const char *name, void *data, struct recursor *r) {
  struct stat st;
  int flags = 0;
  char *old_path = 0;

  if (!name || !r || !r->fn) {
    errno = EINVAL;
    recurse_status = 1;
    return;
  }
  if (r->follow == 'P')
    flags = AT_SYMLINK_NOFOLLOW;
  old_path = r->path;

  if (fstatat(dirfd, name, &st, flags) < 0) {
    if (!(r->flags & SILENT))
      weprintf("%s:", name);
    if (!(r->flags & IGNORE && errno == ENOENT))
      recurse_status = 1;
    return;
  }

  r->path = (char *)name;
  r->fn(dirfd, name, &st, data, r);
  r->path = old_path;
}

void rm(int dirfd, const char *name, struct stat *st, void *data,
        struct recursor *r) {
  int flags = 0;
  (void)data;
  if (st && S_ISDIR(st->st_mode))
    flags = AT_REMOVEDIR;

  if (unlinkat(dirfd, name, flags) < 0) {
    if (!(r && r->flags & SILENT))
      weprintf("%s:", name);
    if (!(r && r->flags & IGNORE && errno == ENOENT))
      rm_status = 1;
  }
}

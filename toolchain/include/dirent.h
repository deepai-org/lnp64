#ifndef LNP64_DIRENT_H
#define LNP64_DIRENT_H

#include <sys/types.h>

typedef struct __lnp64_dir DIR;

struct dirent {
  ino_t d_ino;
  unsigned char d_type;
  char d_name[256];
};

DIR *opendir(const char *name);
struct dirent *readdir(DIR *dirp);
int closedir(DIR *dirp);

#endif

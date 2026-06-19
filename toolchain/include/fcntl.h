#ifndef LNP64_FCNTL_H
#define LNP64_FCNTL_H

#include <sys/types.h>

#define O_RDONLY 0x0000
#define O_WRONLY 0x0001
#define O_RDWR 0x0002
#define O_CREAT 0x0040
#define O_EXCL 0x0080
#define O_TRUNC 0x0200
#define O_APPEND 0x0400
#define O_CLOEXEC 0x80000

#define AT_FDCWD (-100)
#define AT_SYMLINK_NOFOLLOW 0x0100
#define AT_SYMLINK_FOLLOW 0x0400
#define AT_REMOVEDIR 0x0200

int open(const char *path, int flags, ...);
int openat(int dirfd, const char *path, int flags, ...);
int fcntl(int fd, int cmd, ...);

#endif

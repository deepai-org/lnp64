#ifndef LNP64_UNISTD_H
#define LNP64_UNISTD_H

#include <stddef.h>
#include <sys/types.h>

#define STDIN_FILENO 0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

#define F_OK 0
#define X_OK 1
#define W_OK 2
#define R_OK 4

#define _SC_ARG_MAX 0
#define _SC_LOGIN_NAME_MAX 1
#define _POSIX_ARG_MAX 4096

#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

int access(const char *path, int mode);
int chdir(const char *path);
int chown(const char *path, uid_t owner, gid_t group);
int close(int fd);
int execvp(const char *file, char *const argv[]);
void _exit(int status);
pid_t fork(void);
char *getcwd(char *buf, size_t size);
int linkat(int olddirfd, const char *oldpath, int newdirfd,
           const char *newpath, int flags);
off_t lseek(int fd, off_t offset, int whence);
int lchown(const char *path, uid_t owner, gid_t group);
ssize_t read(int fd, void *buf, size_t count);
ssize_t readlink(const char *path, char *buf, size_t bufsiz);
ssize_t readlinkat(int dirfd, const char *path, char *buf, size_t bufsiz);
int rename(const char *oldpath, const char *newpath);
int rmdir(const char *path);
int symlinkat(const char *target, int newdirfd, const char *linkpath);
int unlink(const char *path);
int unlinkat(int dirfd, const char *path, int flags);
long sysconf(int name);
unsigned int sleep(unsigned int seconds);
int symlink(const char *target, const char *linkpath);
ssize_t write(int fd, const void *buf, size_t count);

#endif

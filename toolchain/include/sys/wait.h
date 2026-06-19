#ifndef LNP64_SYS_WAIT_H
#define LNP64_SYS_WAIT_H

#include <sys/types.h>

#define WIFEXITED(status) ((status) < 128)
#define WEXITSTATUS(status) (status)
#define WIFSIGNALED(status) ((status) >= 128)
#define WTERMSIG(status) ((status) >= 128 ? ((status) - 128) : 0)

pid_t wait(int *status);
pid_t waitpid(pid_t pid, int *status, int options);

#endif

#ifndef LNP64_SYS_WAIT_H
#define LNP64_SYS_WAIT_H

#include <sys/types.h>

pid_t waitpid(pid_t pid, int *status, int options);

#endif

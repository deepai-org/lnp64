#ifndef LNP64_SYS_TIMERFD_H
#define LNP64_SYS_TIMERFD_H

#include <time.h>

#define TFD_NONBLOCK 0x800
#define TFD_CLOEXEC 0x80000

int timerfd_create(int clockid, int flags);
int timerfd_settime(int fd, int flags, const struct itimerspec *new_value,
                    struct itimerspec *old_value);
int timerfd_gettime(int fd, struct itimerspec *curr_value);

#endif

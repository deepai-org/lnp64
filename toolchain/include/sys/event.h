#ifndef LNP64_SYS_EVENT_H
#define LNP64_SYS_EVENT_H

#include <time.h>

#define EVFILT_READ (-1)
#define EVFILT_WRITE (-2)

#define EV_ADD 0x0001
#define EV_DELETE 0x0002

struct kevent {
  unsigned long ident;
  short filter;
  unsigned short flags;
  unsigned int fflags;
  long data;
  void *udata;
};

int kqueue(void);
int kevent(int kq, const struct kevent *changelist, int nchanges,
           struct kevent *eventlist, int nevents,
           const struct timespec *timeout);

#endif

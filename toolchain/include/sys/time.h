#ifndef LNP64_SYS_TIME_H
#define LNP64_SYS_TIME_H

#include <time.h>

struct timeval {
  long tv_sec;
  long tv_usec;
};

struct timezone {
  int tz_minuteswest;
  int tz_dsttime;
};

struct itimerval {
  struct timeval it_interval;
  struct timeval it_value;
};

#define ITIMER_REAL    0
#define ITIMER_VIRTUAL 1
#define ITIMER_PROF    2

int gettimeofday(struct timeval *tv, void *tz);
int settimeofday(const struct timeval *tv, void *tz);
int getitimer(int which, struct itimerval *value);
int setitimer(int which, const struct itimerval *value, struct itimerval *ovalue);

#endif

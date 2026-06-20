#ifndef LNP64_TIME_H
#define LNP64_TIME_H

#include <stddef.h>

typedef long time_t;
typedef long clock_t;

#define CLOCKS_PER_SEC 1000000L

struct timespec {
  time_t tv_sec;
  long tv_nsec;
};

struct itimerspec {
  struct timespec it_interval;
  struct timespec it_value;
};

struct tm {
  int tm_sec;
  int tm_min;
  int tm_hour;
  int tm_mday;
  int tm_mon;
  int tm_year;
  int tm_wday;
  int tm_yday;
  int tm_isdst;
  long tm_gmtoff;
  const char *tm_zone;
};

#define CLOCK_REALTIME 0
#define CLOCK_MONOTONIC 1

#define UTIME_NOW ((1L << 30) - 1L)
#define UTIME_OMIT ((1L << 30) - 2L)

time_t time(time_t *timer);
clock_t clock(void);
int clock_gettime(int clockid, struct timespec *tp);
struct tm *localtime(const time_t *timer);
time_t mktime(struct tm *tm);
char *strptime(const char *s, const char *format, struct tm *tm);
size_t strftime(char *s, size_t max, const char *format, const struct tm *tm);

#endif

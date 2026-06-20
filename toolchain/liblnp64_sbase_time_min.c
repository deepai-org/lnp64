#include <errno.h>
#include <limits.h>
#include <stdlib.h>
#include <time.h>

#include "../third_party/sbase/util.h"

static struct tm lnp64_tm;

long long estrtonum(const char *s, long long min, long long max) {
  long long sign = 1;
  long long value = 0;

  if (!s || !*s)
    eprintf("invalid number\n");
  if (*s == '-') {
    sign = -1;
    s++;
  }
  if (!*s)
    eprintf("invalid number\n");
  while (*s) {
    int digit;
    if (*s < '0' || *s > '9')
      eprintf("invalid number\n");
    digit = *s - '0';
    if (value > (LLONG_MAX - digit) / 10)
      eprintf("number out of range\n");
    value = value * 10 + digit;
    s++;
  }
  value = value * sign;
  if (value < min || value > max)
    eprintf("number out of range\n");
  return value;
}

struct tm *localtime(const time_t *timer) {
  time_t value = 0;
  if (timer)
    value = *timer;
  lnp64_tm.tm_sec = (int)(value % 60);
  lnp64_tm.tm_min = (int)((value / 60) % 60);
  lnp64_tm.tm_hour = (int)((value / 3600) % 24);
  lnp64_tm.tm_mday = 1;
  lnp64_tm.tm_mon = 0;
  lnp64_tm.tm_year = 70;
  lnp64_tm.tm_isdst = -1;
  lnp64_tm.tm_gmtoff = 0;
  lnp64_tm.tm_zone = "UTC";
  return &lnp64_tm;
}

time_t mktime(struct tm *tm) {
  time_t value = 0;
  if (!tm) {
    errno = EINVAL;
    return (time_t)-1;
  }
  value += (time_t)tm->tm_sec;
  value += (time_t)tm->tm_min * 60;
  value += (time_t)tm->tm_hour * 3600;
  return value;
}

char *strptime(const char *s, const char *format, struct tm *tm) {
  (void)s;
  (void)format;
  (void)tm;
  errno = EINVAL;
  return 0;
}

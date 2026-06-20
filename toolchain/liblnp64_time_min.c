#include <lnp64/intrinsics.h>

#include <errno.h>
#include <time.h>
#include <sys/time.h>

enum {
  LNP64_OBJECT_KIND_TIMER = 6,
  LNP64_OBJECT_PROFILE_DEFAULT = 0,
  LNP64_TIMER_FLAGS_SUPPORTED = 0
};

static time_t lnp64_realtime_sec(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, REALTIME_SEC" : "=r"(value));
  return (time_t)value;
}

static long lnp64_realtime_nsec(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, REALTIME_NSEC" : "=r"(value));
  return (long)value;
}

int clock_gettime(int clockid, struct timespec *tp) {
  if (!tp) {
    errno = EFAULT;
    return -1;
  }
  if (clockid != CLOCK_REALTIME && clockid != CLOCK_MONOTONIC) {
    errno = EINVAL;
    return -1;
  }
  tp->tv_sec = lnp64_realtime_sec();
  tp->tv_nsec = lnp64_realtime_nsec();
  errno = 0;
  return 0;
}

time_t time(time_t *timer) {
  time_t now = lnp64_realtime_sec();
  if (timer)
    *timer = now;
  return now;
}

clock_t clock(void) {
  /* Approximate processor time as elapsed realtime in CLOCKS_PER_SEC ticks. */
  return (clock_t)lnp64_realtime_sec() * CLOCKS_PER_SEC +
         (clock_t)(lnp64_realtime_nsec() / 1000L);
}

int usleep(unsigned int usec) {
  (void)usec;
  __lnp_yield();
  return 0;
}

unsigned int sleep(unsigned int seconds) {
  for (unsigned int i = 0; i < seconds; i = i + 1)
    __lnp_yield();
  return 0;
}

int timerfd_create(int clockid, int flags) {
  if (clockid != CLOCK_REALTIME && clockid != CLOCK_MONOTONIC) {
    errno = EINVAL;
    return -1;
  }
  if ((flags & ~LNP64_TIMER_FLAGS_SUPPORTED) != 0) {
    errno = EINVAL;
    return -1;
  }

  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_CTL_CREATE;
  record[1] = LNP64_OBJECT_KIND_TIMER;
  record[2] = LNP64_OBJECT_PROFILE_DEFAULT;
  record[3] = 0;
  record[4] = 0;
  record[5] = 0;
  record[6] = 0;
  record[7] = 0;
  record[8] = 0;
  long status = (long)__lnp_object_ctl((lnp64_word_t)record);
  if (status < 0) {
    errno = (int)-status;
    return -1;
  }
  return (int)record[3];
}

static lnp64_word_t lnp64_timerfd_ticks(const struct itimerspec *spec) {
  if (!spec)
    return 0;
  if (spec->it_value.tv_sec <= 0 && spec->it_value.tv_nsec <= 0)
    return 0;
  return 1;
}

static void lnp64_zero_itimerspec(struct itimerspec *spec) {
  if (!spec)
    return;
  spec->it_interval.tv_sec = 0;
  spec->it_interval.tv_nsec = 0;
  spec->it_value.tv_sec = 0;
  spec->it_value.tv_nsec = 0;
}

int timerfd_settime(int fd, int flags, const struct itimerspec *new_value,
                    struct itimerspec *old_value) {
  if (flags != 0) {
    errno = EINVAL;
    return -1;
  }
  if (!new_value) {
    errno = EFAULT;
    return -1;
  }
  lnp64_zero_itimerspec(old_value);
  lnp64_word_t ticks = lnp64_timerfd_ticks(new_value);
  if (__lnp_push((lnp64_cap_t)(unsigned int)fd, (lnp64_word_t)&ticks,
                 sizeof(ticks)) != sizeof(ticks)) {
    errno = EBADF;
    return -1;
  }
  errno = 0;
  return 0;
}

int timerfd_gettime(int fd, struct itimerspec *curr_value) {
  (void)fd;
  if (!curr_value) {
    errno = EFAULT;
    return -1;
  }
  lnp64_zero_itimerspec(curr_value);
  errno = 0;
  return 0;
}

static struct tm lnp64_static_tm_result;

struct tm *gmtime(const time_t *timer) {
  if (!timer)
    return 0;
  time_t t = *timer;
  /* Simple conversion: days since epoch are t / 86400 */
  time_t days = t / 86400;
  time_t secs_today = t % 86400;
  int hour = (int)(secs_today / 3600);
  int min = (int)((secs_today % 3600) / 60);
  int sec = (int)(secs_today % 60);

  /* Approximate day-of-year and year (ignores leap seconds) */
  int year = 1970;
  int days_left = (int)days;
  while (days_left >= 365) {
    if ((year % 4 == 0 && year % 100 != 0) || year % 400 == 0)
      days_left -= 366;
    else
      days_left -= 365;
    year++;
  }

  int month = 0;
  int mdays[] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
  if ((year % 4 == 0 && year % 100 != 0) || year % 400 == 0)
    mdays[1] = 29;

  int day = days_left;
  for (month = 0; month < 12 && day >= mdays[month]; month++)
    day -= mdays[month];

  lnp64_static_tm_result.tm_sec = sec;
  lnp64_static_tm_result.tm_min = min;
  lnp64_static_tm_result.tm_hour = hour;
  lnp64_static_tm_result.tm_mday = day + 1;
  lnp64_static_tm_result.tm_mon = month;
  lnp64_static_tm_result.tm_year = year - 1900;
  lnp64_static_tm_result.tm_yday = (int)days_left;
  lnp64_static_tm_result.tm_wday = (int)(days % 7);
  lnp64_static_tm_result.tm_isdst = 0;
  lnp64_static_tm_result.tm_gmtoff = 0;
  lnp64_static_tm_result.tm_zone = "GMT";
  return &lnp64_static_tm_result;
}

struct tm *localtime(const time_t *timer) {
  /* For now, localtime is the same as gmtime (no timezone support) */
  return gmtime(timer);
}

time_t mktime(struct tm *tm) {
  if (!tm)
    return (time_t)-1;
  /* Simplified: calculate days since epoch */
  time_t days = 0;
  int year = 1970;
  int target_year = tm->tm_year + 1900;
  while (year < target_year) {
    if ((year % 4 == 0 && year % 100 != 0) || year % 400 == 0)
      days += 366;
    else
      days += 365;
    year++;
  }
  /* Add days for months */
  int mdays[] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
  if ((target_year % 4 == 0 && target_year % 100 != 0) || target_year % 400 == 0)
    mdays[1] = 29;
  for (int m = 0; m < tm->tm_mon && m < 12; m++)
    days += mdays[m];
  days += tm->tm_mday - 1;
  /* Add hours, minutes, seconds */
  time_t secs = days * 86400 + tm->tm_hour * 3600 + tm->tm_min * 60 + tm->tm_sec;
  return secs;
}

double difftime(time_t time1, time_t time0) {
  return (double)(time1 - time0);
}

int system(const char *command) {
  (void)command;
  return -1;
}

size_t strftime(char *s, size_t max, const char *format, const struct tm *tm) {
  /* Minimal strftime: just handle %Y-%m-%d type formats */
  (void)tm;
  if (!s || max == 0)
    return 0;
  if (!format) {
    s[0] = 0;
    return 0;
  }
  /* Simple stub: just copy format if no % specifiers, else return empty */
  if (!__builtin_strchr(format, '%')) {
    size_t len = 0;
    while (format[len] && len < max - 1) {
      s[len] = format[len];
      len++;
    }
    s[len] = 0;
    return len;
  }
  /* For now, return a simple date string */
  const char *result = "2026-01-01";
  size_t len = 0;
  while (result[len] && len < max - 1) {
    s[len] = result[len];
    len++;
  }
  s[len] = 0;
  return len;
}

int gettimeofday(struct timeval *tv, void *tz) {
  (void)tz;
  if (!tv)
    return -1;
  tv->tv_sec = lnp64_realtime_sec();
  tv->tv_usec = lnp64_realtime_nsec() / 1000;
  return 0;
}

int settimeofday(const struct timeval *tv, void *tz) {
  (void)tv;
  (void)tz;
  return -1;
}

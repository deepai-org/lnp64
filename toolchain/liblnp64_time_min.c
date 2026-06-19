#include <errno.h>
#include <time.h>

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

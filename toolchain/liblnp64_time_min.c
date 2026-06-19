#include "lnp64_intrinsics.h"

#include <errno.h>
#include <time.h>

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

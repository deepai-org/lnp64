#include "lnp64_intrinsics.h"

#include <poll.h>
#include <signal.h>
#include <sys/timerfd.h>
#include <time.h>
#include <unistd.h>

static volatile int alarm_seen;

static void on_alarm(int signum) {
  if (signum == SIGALRM)
    alarm_seen = 1;
  __lnp_sigret();
}

static int check_alarm(void) {
  unsigned int previous;

  alarm_seen = 0;
  if (signal(SIGALRM, on_alarm) != 0)
    return 1;
  previous = alarm(1);
  if (previous != 0)
    return 2;
  previous = alarm(2);
  if (previous == 0)
    return 3;
  alarm(1);
  for (int i = 0; i < 200 && alarm_seen == 0; i = i + 1)
    __lnp_yield();
  return alarm_seen == 1 ? 0 : 4;
}

static int check_timerfd(void) {
  int fd = timerfd_create(CLOCK_MONOTONIC, 0);
  if (fd == -1)
    return 1;

  struct itimerspec spec = {
      .it_interval = {.tv_sec = 0, .tv_nsec = 0},
      .it_value = {.tv_sec = 0, .tv_nsec = 1},
  };
  struct itimerspec old = {
      .it_interval = {.tv_sec = 9, .tv_nsec = 9},
      .it_value = {.tv_sec = 9, .tv_nsec = 9},
  };
  if (timerfd_settime(fd, 0, &spec, &old) != 0)
    return 2;
  if (old.it_value.tv_sec != 0 || old.it_value.tv_nsec != 0)
    return 3;

  struct pollfd pfd = {.fd = fd, .events = POLLIN, .revents = 0};
  for (int i = 0; i < 16; i = i + 1)
    __lnp_yield();
  if (poll(&pfd, 1, 0) != 1)
    return 4;
  if (pfd.revents != POLLIN)
    return 5;

  unsigned long expirations = 0;
  if (read(fd, &expirations, sizeof(expirations)) != (long)sizeof(expirations))
    return 6;
  if (expirations != 1)
    return 7;

  spec.it_value.tv_sec = 5;
  spec.it_value.tv_nsec = 0;
  if (timerfd_gettime(fd, &spec) != 0)
    return 8;
  if (spec.it_value.tv_sec != 0 || spec.it_value.tv_nsec != 0)
    return 9;

  spec.it_value.tv_sec = 0;
  spec.it_value.tv_nsec = 0;
  if (timerfd_settime(fd, 0, &spec, 0) != 0)
    return 10;
  return 0;
}

int main(void) {
  if (usleep(1) != 0)
    return 1;
  int rc = check_alarm();
  if (rc != 0)
    return 10 + rc;
  rc = check_timerfd();
  if (rc != 0)
    return 30 + rc;

  write(1, "timer_test ok\n", 14);
  return 0;
}

#include "lnp64_intrinsics.h"

typedef unsigned long nfds_t;

typedef struct {
  unsigned long bits[16];
} fd_set;

struct timeval {
  long tv_sec;
  long tv_usec;
};

struct pollfd {
  int fd;
  short events;
  short revents;
};

enum {
  LNP64_POLLIN = 0x01,
  LNP64_POLLOUT = 0x04,
  LNP64_POLLERR = 0x08,
  LNP64_POLLNVAL = 0x20,
};

static lnp64_word_t lnp64_poll_timeout_word(int timeout) {
  if (timeout < 0)
    return (lnp64_word_t)-1;
  return (lnp64_word_t)(unsigned int)timeout;
}

int poll(struct pollfd *fds, nfds_t nfds, int timeout) {
  int ready = 0;
  lnp64_word_t timeout_word = lnp64_poll_timeout_word(timeout);

  if (!fds && nfds != 0)
    return -1;

  for (nfds_t i = 0; i < nfds; i = i + 1) {
    lnp64_word_t events = (lnp64_word_t)(unsigned short)fds[i].events;
    fds[i].revents = 0;
    if (fds[i].fd < 0)
      continue;
    if (events == 0)
      continue;
    lnp64_word_t status =
        __lnp_await((lnp64_cap_t)(unsigned long)fds[i].fd, events, timeout_word);
    if (status == (lnp64_word_t)-1) {
      fds[i].revents = LNP64_POLLNVAL;
      ready = ready + 1;
    } else if (status != 0) {
      fds[i].revents = (short)events;
      ready = ready + 1;
    }
  }

  return ready;
}

static int lnp64_fd_isset(int fd, const fd_set *set) {
  if (!set || fd < 0 || fd >= 1024)
    return 0;
  unsigned long word = (unsigned long)fd / 64;
  unsigned long bit = (unsigned long)fd % 64;
  return (set->bits[word] & (1UL << bit)) != 0;
}

static void lnp64_fd_clear(int fd, fd_set *set) {
  if (!set || fd < 0 || fd >= 1024)
    return;
  unsigned long word = (unsigned long)fd / 64;
  unsigned long bit = (unsigned long)fd % 64;
  set->bits[word] &= ~(1UL << bit);
}

static lnp64_word_t lnp64_select_timeout_word(const struct timeval *timeout) {
  if (!timeout)
    return (lnp64_word_t)-1;
  if (timeout->tv_sec <= 0 && timeout->tv_usec <= 0)
    return 0;
  return 1;
}

int select(int nfds, fd_set *readfds, fd_set *writefds, fd_set *exceptfds,
           struct timeval *timeout) {
  int ready = 0;
  lnp64_word_t timeout_word = lnp64_select_timeout_word(timeout);

  if (nfds < 0 || nfds > 1024)
    return -1;

  for (int fd = 0; fd < nfds; fd = fd + 1) {
    lnp64_word_t events = 0;
    if (lnp64_fd_isset(fd, readfds))
      events |= LNP64_POLLIN;
    if (lnp64_fd_isset(fd, writefds))
      events |= LNP64_POLLOUT;
    if (lnp64_fd_isset(fd, exceptfds))
      events |= LNP64_POLLERR;
    if (events == 0)
      continue;

    lnp64_word_t status = __lnp_await((lnp64_cap_t)(unsigned long)fd, events,
                                      timeout_word);
    if (status == 0 || status == (lnp64_word_t)-1) {
      lnp64_fd_clear(fd, readfds);
      lnp64_fd_clear(fd, writefds);
      lnp64_fd_clear(fd, exceptfds);
    } else {
      ready = ready + 1;
    }
  }

  return ready;
}

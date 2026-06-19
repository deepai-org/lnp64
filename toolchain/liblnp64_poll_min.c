#include "lnp64_intrinsics.h"

typedef unsigned long nfds_t;

struct pollfd {
  int fd;
  short events;
  short revents;
};

enum {
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

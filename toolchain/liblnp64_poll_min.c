#include <lnp64/intrinsics.h>

#include <poll.h>
#include <sys/epoll.h>
#include <sys/event.h>
#include <sys/select.h>

enum {
  LNP64_EVFILT_READ = -1,
  LNP64_EVFILT_WRITE = -2,
  LNP64_EV_ADD = 0x0001,
  LNP64_EV_DELETE = 0x0002,
  LNP64_EV_ONESHOT = 0x0010,
  LNP64_EPOLL_CTL_ADD = 1,
  LNP64_EPOLL_CTL_DEL = 2,
  LNP64_EPOLL_CTL_MOD = 3,
  LNP64_EPOLL_FD = 64,
  LNP64_EPOLL_MAX = 16,
  LNP64_KQUEUE_FD = 65,
  LNP64_KQUEUE_MAX = 16,
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
    } else if (status == 0) {
      fds[i].revents = (short)events;
      ready = ready + 1;
    }
  }

  return ready;
}

static int lnp64_filter_events(short filter) {
  if (filter == LNP64_EVFILT_READ)
    return LNP64_POLLIN;
  if (filter == LNP64_EVFILT_WRITE)
    return LNP64_POLLOUT;
  return 0;
}

static lnp64_word_t lnp64_timespec_timeout_word(const struct timespec *timeout) {
  if (!timeout)
    return (lnp64_word_t)-1;
  if (timeout->tv_sec <= 0 && timeout->tv_nsec <= 0)
    return 0;
  return 1;
}

static int lnp64_epoll_created;
static int lnp64_epoll_count;
static int lnp64_epoll_fd[LNP64_EPOLL_MAX];
static unsigned int lnp64_epoll_events[LNP64_EPOLL_MAX];
static unsigned long lnp64_epoll_data[LNP64_EPOLL_MAX];

int epoll_create1(int flags) {
  if (flags != 0)
    return -1;
  lnp64_epoll_created = 1;
  lnp64_epoll_count = 0;
  return LNP64_EPOLL_FD;
}

static int lnp64_epoll_find(int fd) {
  for (int i = 0; i < lnp64_epoll_count; i = i + 1) {
    if (lnp64_epoll_fd[i] == fd)
      return i;
  }
  return -1;
}

int epoll_ctl(int epfd, int op, int fd, struct epoll_event *event) {
  if (!lnp64_epoll_created || epfd != LNP64_EPOLL_FD || fd < 0)
    return -1;
  int slot = lnp64_epoll_find(fd);
  if (op == LNP64_EPOLL_CTL_ADD) {
    if (!event || slot >= 0 || lnp64_epoll_count >= LNP64_EPOLL_MAX)
      return -1;
    slot = lnp64_epoll_count;
    lnp64_epoll_count = lnp64_epoll_count + 1;
    lnp64_epoll_fd[slot] = fd;
    lnp64_epoll_events[slot] = event->events;
    lnp64_epoll_data[slot] = event->data;
    return 0;
  }
  if (op == LNP64_EPOLL_CTL_MOD) {
    if (!event || slot < 0)
      return -1;
    lnp64_epoll_events[slot] = event->events;
    lnp64_epoll_data[slot] = event->data;
    return 0;
  }
  if (op == LNP64_EPOLL_CTL_DEL) {
    if (slot < 0)
      return -1;
    int last = lnp64_epoll_count - 1;
    lnp64_epoll_fd[slot] = lnp64_epoll_fd[last];
    lnp64_epoll_events[slot] = lnp64_epoll_events[last];
    lnp64_epoll_data[slot] = lnp64_epoll_data[last];
    lnp64_epoll_count = last;
    return 0;
  }
  return -1;
}

int epoll_wait(int epfd, struct epoll_event *events, int maxevents,
               int timeout) {
  int ready = 0;
  lnp64_word_t timeout_word = lnp64_poll_timeout_word(timeout);

  if (!lnp64_epoll_created || epfd != LNP64_EPOLL_FD || !events ||
      maxevents < 0)
    return -1;

  for (int i = 0; i < lnp64_epoll_count && ready < maxevents; i = i + 1) {
    lnp64_word_t mask = (lnp64_word_t)lnp64_epoll_events[i];
    if (mask == 0)
      continue;
    lnp64_word_t status = __lnp_await(
        (lnp64_cap_t)(unsigned long)lnp64_epoll_fd[i], mask, timeout_word);
    if (status == 0) {
      events[ready].events = lnp64_epoll_events[i];
      events[ready].data = lnp64_epoll_data[i];
      ready = ready + 1;
    }
  }

  return ready;
}

static int lnp64_kqueue_created;
static int lnp64_kqueue_count;
static unsigned long lnp64_kqueue_ident[LNP64_KQUEUE_MAX];
static short lnp64_kqueue_filter[LNP64_KQUEUE_MAX];
static unsigned short lnp64_kqueue_flags[LNP64_KQUEUE_MAX];
static void *lnp64_kqueue_udata[LNP64_KQUEUE_MAX];

int kqueue(void) {
  lnp64_kqueue_created = 1;
  lnp64_kqueue_count = 0;
  return LNP64_KQUEUE_FD;
}

static int lnp64_kqueue_find(unsigned long ident, short filter) {
  for (int i = 0; i < lnp64_kqueue_count; i = i + 1) {
    if (lnp64_kqueue_ident[i] == ident && lnp64_kqueue_filter[i] == filter)
      return i;
  }
  return -1;
}

static void lnp64_kqueue_remove_slot(int slot) {
  int last = lnp64_kqueue_count - 1;
  lnp64_kqueue_ident[slot] = lnp64_kqueue_ident[last];
  lnp64_kqueue_filter[slot] = lnp64_kqueue_filter[last];
  lnp64_kqueue_flags[slot] = lnp64_kqueue_flags[last];
  lnp64_kqueue_udata[slot] = lnp64_kqueue_udata[last];
  lnp64_kqueue_count = last;
}

static int lnp64_kqueue_apply_change(const struct kevent *change) {
  int slot = lnp64_kqueue_find(change->ident, change->filter);
  if (change->flags & LNP64_EV_DELETE) {
    if (slot < 0)
      return -1;
    lnp64_kqueue_remove_slot(slot);
    return 0;
  }
  if (!(change->flags & LNP64_EV_ADD))
    return -1;
  if (slot < 0) {
    if (lnp64_kqueue_count >= LNP64_KQUEUE_MAX)
      return -1;
    slot = lnp64_kqueue_count;
    lnp64_kqueue_count = lnp64_kqueue_count + 1;
  }
  lnp64_kqueue_ident[slot] = change->ident;
  lnp64_kqueue_filter[slot] = change->filter;
  lnp64_kqueue_flags[slot] = change->flags;
  lnp64_kqueue_udata[slot] = change->udata;
  return 0;
}

int kevent(int kq, const struct kevent *changelist, int nchanges,
           struct kevent *eventlist, int nevents,
           const struct timespec *timeout) {
  int ready = 0;
  lnp64_word_t timeout_word = lnp64_timespec_timeout_word(timeout);

  if (!lnp64_kqueue_created || kq != LNP64_KQUEUE_FD || nchanges < 0 ||
      nevents < 0)
    return -1;

  for (int i = 0; i < nchanges; i = i + 1) {
    if (!changelist || lnp64_kqueue_apply_change(&changelist[i]) != 0)
      return -1;
  }

  if (!eventlist || nevents == 0)
    return 0;

  for (int i = 0; i < lnp64_kqueue_count && ready < nevents; i = i + 1) {
    lnp64_word_t mask = (lnp64_word_t)lnp64_filter_events(lnp64_kqueue_filter[i]);
    if (mask == 0)
      continue;
    lnp64_word_t status =
        __lnp_await((lnp64_cap_t)lnp64_kqueue_ident[i], mask, timeout_word);
    if (status == 0) {
      eventlist[ready].ident = lnp64_kqueue_ident[i];
      eventlist[ready].filter = lnp64_kqueue_filter[i];
      eventlist[ready].flags = 0;
      eventlist[ready].fflags = 0;
      eventlist[ready].data = 0;
      eventlist[ready].udata = lnp64_kqueue_udata[i];
      ready = ready + 1;
      if (lnp64_kqueue_flags[i] & LNP64_EV_ONESHOT) {
        lnp64_kqueue_remove_slot(i);
        i = i - 1;
      }
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
    if (status == (lnp64_word_t)-1) {
      lnp64_fd_clear(fd, readfds);
      lnp64_fd_clear(fd, writefds);
      lnp64_fd_clear(fd, exceptfds);
    } else if (status == 0) {
      ready = ready + 1;
    } else {
      lnp64_fd_clear(fd, readfds);
      lnp64_fd_clear(fd, writefds);
      lnp64_fd_clear(fd, exceptfds);
    }
  }

  return ready;
}

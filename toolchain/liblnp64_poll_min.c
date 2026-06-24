#include <lnp64/intrinsics.h>

#include <unistd.h>
#include <poll.h>
#include <sys/epoll.h>
#include <sys/event.h>
#include <sys/select.h>

enum {
  LNP64_EVFILT_READ = -1,
  LNP64_EVFILT_WRITE = -2,
  LNP64_EVFILT_USER = -10,
  LNP64_EV_ADD = 0x0001,
  LNP64_EV_DELETE = 0x0002,
  LNP64_EV_ENABLE = 0x0004,
  LNP64_EV_DISABLE = 0x0008,
  LNP64_EV_ONESHOT = 0x0010,
  LNP64_EV_RECEIPT = 0x0040,
  LNP64_EV_ERROR = 0x4000,
  LNP64_NOTE_TRIGGER = 0x01000000,
  LNP64_EPOLL_CTL_ADD = 1,
  LNP64_EPOLL_CTL_DEL = 2,
  LNP64_EPOLL_CTL_MOD = 3,
  LNP64_ENOENT = 2,
  LNP64_EINVAL = 22,
  LNP64_ENOMEM = 12,
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

enum { LNP64_POLL_WAIT_MAX = 64 };

/* EP-I-full await retirement: per-fd readiness via the unified `wait` verb,
   replacing the legacy per-fd __lnp_await probe. Builds a 1-entry { handle,
   events, revents } waitset and returns the revents the verb writes back
   (POLLNVAL for a bad/unpollable fd). timeout=0 polls; nonzero blocks until an
   edge or the deadline — exactly the await(fd,mask,timeout) contract. */
static lnp64_word_t lnp64_wait_one(lnp64_word_t fd, lnp64_word_t events,
                                   lnp64_word_t timeout) {
  lnp64_word_t entry[3];   /* {handle, events, revents} */
  lnp64_word_t waitset[2]; /* {entries_ptr, count} */
  entry[0] = fd;
  entry[1] = events;
  entry[2] = 0;
  waitset[0] = (lnp64_word_t)entry;
  waitset[1] = 1;
  (void)__lnp_wait((lnp64_word_t)waitset, timeout);
  return entry[2];
}

int poll(struct pollfd *fds, nfds_t nfds, int timeout) {
  if (!fds && nfds != 0)
    return -1;

  /* EP-H: route poll through the unified `wait` verb — a batched poll over a
     24-byte { handle, events, revents } waitset. The verb scans readiness with
     the same edge sources as the legacy per-fd await (identical revents) and
     blocks natively on a non-zero timeout. Purely additive: the legacy await
     opcode stays live; only which verb the shim calls changes. */
  if (nfds <= LNP64_POLL_WAIT_MAX) {
    lnp64_word_t entries[LNP64_POLL_WAIT_MAX * 3]; /* {handle,events,revents} */
    lnp64_word_t waitset[2];
    nfds_t map[LNP64_POLL_WAIT_MAX];
    nfds_t valid = 0;
    for (nfds_t i = 0; i < nfds; i++) {
      lnp64_word_t events = (lnp64_word_t)(unsigned short)fds[i].events;
      fds[i].revents = 0;
      if (fds[i].fd < 0 || events == 0)
        continue; /* skipped fds keep revents 0 and stay out of the waitset */
      entries[valid * 3 + 0] = (lnp64_word_t)(unsigned long)fds[i].fd;
      entries[valid * 3 + 1] = events;
      entries[valid * 3 + 2] = 0;
      map[valid] = i;
      valid++;
    }
    waitset[0] = (lnp64_word_t)entries;
    waitset[1] = (lnp64_word_t)valid;
    (void)__lnp_wait((lnp64_word_t)waitset, lnp64_poll_timeout_word(timeout));
    int ready = 0;
    for (nfds_t k = 0; k < valid; k++) {
      lnp64_word_t rv = entries[k * 3 + 2];
      if (rv != 0) {
        fds[map[k]].revents = (short)rv;
        ready++;
      }
    }
    return ready;
  }

  /* Fallback for pathologically large sets: legacy per-fd await loop. */
  int ready = 0;
  for (nfds_t i = 0; i < nfds; i++) {
    lnp64_word_t events = (lnp64_word_t)(unsigned short)fds[i].events;
    fds[i].revents = 0;
    if (fds[i].fd < 0 || events == 0)
      continue;
    lnp64_word_t rv = lnp64_wait_one((lnp64_word_t)(unsigned long)fds[i].fd, events, 0);
    if (rv != 0) { /* revents incl. POLLNVAL — a returned event counts as ready */
      fds[i].revents = (short)rv;
      ready++;
    }
  }
  if (ready > 0 || timeout == 0)
    return ready;
  for (;;) {
    __lnp_sleep_1tick();
    int n = 0;
    for (nfds_t j = 0; j < nfds; j++) {
      lnp64_word_t ev = (lnp64_word_t)(unsigned short)fds[j].events;
      fds[j].revents = 0;
      if (fds[j].fd < 0 || ev == 0) continue;
      lnp64_word_t rv = lnp64_wait_one((lnp64_word_t)(unsigned long)fds[j].fd, ev, 0);
      if (rv != 0) { fds[j].revents = (short)rv; n++; }
    }
    if (n > 0) return n;
  }
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

int epoll_create(int size) {
  (void)size;
  lnp64_epoll_created = 1;
  lnp64_epoll_count = 0;
  return LNP64_EPOLL_FD;
}

int epoll_create1(int flags) {
  (void)flags;
  return epoll_create(0);
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
  if (!lnp64_epoll_created || epfd != LNP64_EPOLL_FD || !events ||
      maxevents < 0)
    return -1;

  /* EP-H: route epoll_wait — Redis's readiness path — through the unified
     `wait` verb. Build a waitset over the registered fds and let the verb scan
     readiness (same edge sources as the legacy per-fd await) and block natively
     on a non-zero timeout. Ready entries report their registered events/data,
     matching the prior shim. Purely additive: the legacy await stays live. */
  lnp64_word_t entries[LNP64_EPOLL_MAX * 3]; /* {handle,events,revents} */
  lnp64_word_t waitset[2];
  int idx[LNP64_EPOLL_MAX];
  int valid = 0;
  for (int i = 0; i < lnp64_epoll_count; i++) {
    lnp64_word_t mask = (lnp64_word_t)lnp64_epoll_events[i];
    if (mask == 0) continue;
    entries[valid * 3 + 0] = (lnp64_word_t)(unsigned long)lnp64_epoll_fd[i];
    entries[valid * 3 + 1] = mask;
    entries[valid * 3 + 2] = 0;
    idx[valid] = i;
    valid++;
  }
  waitset[0] = (lnp64_word_t)entries;
  waitset[1] = (lnp64_word_t)valid;
  (void)__lnp_wait((lnp64_word_t)waitset, lnp64_poll_timeout_word(timeout));
  int ready = 0;
  for (int k = 0; k < valid && ready < maxevents; k++) {
    lnp64_word_t rv = entries[k * 3 + 2];
    if (rv != 0) {
      events[ready].events = lnp64_epoll_events[idx[k]];
      events[ready].data = lnp64_epoll_data[idx[k]];
      ready++;
    }
  }
  return ready;
}

static int lnp64_kqueue_created;
static int lnp64_kqueue_count;
static unsigned long lnp64_kqueue_ident[LNP64_KQUEUE_MAX];
static short lnp64_kqueue_filter[LNP64_KQUEUE_MAX];
static unsigned short lnp64_kqueue_flags[LNP64_KQUEUE_MAX];
static unsigned int lnp64_kqueue_fflags[LNP64_KQUEUE_MAX];
static void *lnp64_kqueue_udata[LNP64_KQUEUE_MAX];

int kqueue(void) {
  lnp64_kqueue_created = 1;
  lnp64_kqueue_count = 0;
  return LNP64_KQUEUE_FD;
}

int lnp64_kqueue_close(int fd) {
  if (fd != LNP64_KQUEUE_FD)
    return -2;
  if (!lnp64_kqueue_created)
    return -1;
  lnp64_kqueue_created = 0;
  lnp64_kqueue_count = 0;
  return 0;
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
  lnp64_kqueue_fflags[slot] = lnp64_kqueue_fflags[last];
  lnp64_kqueue_udata[slot] = lnp64_kqueue_udata[last];
  lnp64_kqueue_count = last;
}

static int lnp64_kqueue_filter_supported(short filter) {
  return filter == LNP64_EVFILT_READ || filter == LNP64_EVFILT_WRITE ||
         filter == LNP64_EVFILT_USER;
}

static int lnp64_kqueue_apply_change(const struct kevent *change) {
  if (!lnp64_kqueue_filter_supported(change->filter))
    return LNP64_EINVAL;
  int slot = lnp64_kqueue_find(change->ident, change->filter);
  if (change->flags & LNP64_EV_DELETE) {
    if (slot < 0)
      return LNP64_ENOENT;
    lnp64_kqueue_remove_slot(slot);
    return 0;
  }
  if (slot >= 0 && (change->flags & (LNP64_EV_ENABLE | LNP64_EV_DISABLE))) {
    if (change->flags & LNP64_EV_DISABLE)
      lnp64_kqueue_flags[slot] |= LNP64_EV_DISABLE;
    if (change->flags & LNP64_EV_ENABLE)
      lnp64_kqueue_flags[slot] &= (unsigned short)~LNP64_EV_DISABLE;
    if (!(change->flags & LNP64_EV_ADD))
      return 0;
  }
  if (!(change->flags & LNP64_EV_ADD))
    return LNP64_EINVAL;
  if (slot < 0) {
    if (lnp64_kqueue_count >= LNP64_KQUEUE_MAX)
      return LNP64_ENOMEM;
    slot = lnp64_kqueue_count;
    lnp64_kqueue_count = lnp64_kqueue_count + 1;
  }
  lnp64_kqueue_ident[slot] = change->ident;
  lnp64_kqueue_filter[slot] = change->filter;
  lnp64_kqueue_flags[slot] = change->flags;
  if (change->filter == LNP64_EVFILT_USER &&
      (change->fflags & LNP64_NOTE_TRIGGER))
    lnp64_kqueue_fflags[slot] |= LNP64_NOTE_TRIGGER;
  else
    lnp64_kqueue_fflags[slot] = change->fflags;
  lnp64_kqueue_udata[slot] = change->udata;
  return 0;
}

static void lnp64_kqueue_write_error(const struct kevent *change,
                                     struct kevent *event, int error) {
  event->ident = change->ident;
  event->filter = change->filter;
  event->flags = LNP64_EV_ERROR;
  event->fflags = 0;
  event->data = error;
  event->udata = change->udata;
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
    int change_error;
    if (!changelist)
      return -1;
    change_error = lnp64_kqueue_apply_change(&changelist[i]);
    if (change_error != 0 || (changelist[i].flags & LNP64_EV_RECEIPT)) {
      if (!eventlist || ready >= nevents)
        return -1;
      lnp64_kqueue_write_error(&changelist[i], &eventlist[ready], change_error);
      ready = ready + 1;
    }
  }

  if (!eventlist || nevents == 0)
    return ready;

  for (int i = 0; i < lnp64_kqueue_count && ready < nevents; i = i + 1) {
    lnp64_word_t mask = (lnp64_word_t)lnp64_filter_events(lnp64_kqueue_filter[i]);
    if (lnp64_kqueue_flags[i] & LNP64_EV_DISABLE)
      continue;
    if (lnp64_kqueue_filter[i] == LNP64_EVFILT_USER) {
      if (!(lnp64_kqueue_fflags[i] & LNP64_NOTE_TRIGGER))
        continue;
      eventlist[ready].ident = lnp64_kqueue_ident[i];
      eventlist[ready].filter = lnp64_kqueue_filter[i];
      eventlist[ready].flags = 0;
      eventlist[ready].fflags = LNP64_NOTE_TRIGGER;
      eventlist[ready].data = 0;
      eventlist[ready].udata = lnp64_kqueue_udata[i];
      lnp64_kqueue_fflags[i] &= ~LNP64_NOTE_TRIGGER;
      ready = ready + 1;
      if (lnp64_kqueue_flags[i] & LNP64_EV_ONESHOT) {
        lnp64_kqueue_remove_slot(i);
        i = i - 1;
      }
      continue;
    }
    if (mask == 0)
      continue;
    lnp64_word_t rv =
        lnp64_wait_one((lnp64_word_t)lnp64_kqueue_ident[i], mask, timeout_word);
    if ((rv & mask) != 0) { /* ready for the requested filter (was await status==0) */
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
  lnp64_word_t timeout_word = lnp64_select_timeout_word(timeout);

  if (nfds < 0 || nfds > 1024)
    return -1;

  /* Save original sets — non-blocking scan clears non-ready fds in place,
   * and we need the originals for the blocking retry loop. */
  fd_set orig_rfds, orig_wfds;
  if (readfds)  orig_rfds = *readfds;
  if (writefds) orig_wfds = *writefds;

  /* Non-blocking scan via the wait verb (lnp64_wait_one); revents>0 = ready. */
  int ready = 0;
  for (int fd = 0; fd < nfds; fd = fd + 1) {
    lnp64_word_t events = 0;
    if (readfds  && lnp64_fd_isset(fd, readfds))  events |= LNP64_POLLIN;
    if (writefds && lnp64_fd_isset(fd, writefds)) events |= LNP64_POLLOUT;
    if (exceptfds && lnp64_fd_isset(fd, exceptfds)) events |= LNP64_POLLERR;
    if (events == 0)
      continue;
    lnp64_word_t rv = lnp64_wait_one((lnp64_word_t)(unsigned long)fd, events, 0);
    if ((rv & events) != 0) { /* ready for a requested event (POLLNVAL excluded) */
      ready = ready + 1;
    } else {
      if (readfds)  lnp64_fd_clear(fd, readfds);
      if (writefds) lnp64_fd_clear(fd, writefds);
      if (exceptfds) lnp64_fd_clear(fd, exceptfds);
    }
  }
  if (ready > 0 || timeout_word == 0)
    return ready;

  /* Blocking phase: sleep 1 tick (~10ms) and rescan all fds non-blocking.
   * This correctly handles select() with multiple fds (e.g. a pipe + a
   * TcpListener) without blocking forever on whichever happens to come first
   * in the fd set. */
  for (;;) {
    __lnp_sleep_1tick();
    if (readfds)  *readfds  = orig_rfds;
    if (writefds) *writefds = orig_wfds;
    int n = 0;
    for (int j = 0; j < nfds; j = j + 1) {
      lnp64_word_t ev = 0;
      if (readfds  && lnp64_fd_isset(j, &orig_rfds)) ev |= LNP64_POLLIN;
      if (writefds && lnp64_fd_isset(j, &orig_wfds)) ev |= LNP64_POLLOUT;
      if (ev == 0)
        continue;
      lnp64_word_t s = lnp64_wait_one((lnp64_word_t)(unsigned long)j, ev, 0);
      if ((s & ev) != 0) {
        n = n + 1;
      } else {
        if (readfds)  lnp64_fd_clear(j, readfds);
        if (writefds) lnp64_fd_clear(j, writefds);
        if (exceptfds) lnp64_fd_clear(j, exceptfds);
      }
    }
    if (n > 0)
      return n;
  }
}

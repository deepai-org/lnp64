#include "lnp64_intrinsics.h"

typedef unsigned long sigset_t;
typedef void (*lnp64_sighandler_t)(int);

struct sigaction {
  lnp64_sighandler_t sa_handler;
  sigset_t sa_mask;
  int sa_flags;
};

enum {
  LNP64_SIG_BLOCK = 0,
  LNP64_SIG_UNBLOCK = 1,
  LNP64_SIG_SETMASK = 2
};

lnp64_sighandler_t signal(int signum, lnp64_sighandler_t handler) {
  __lnp_sigaction((lnp64_word_t)(unsigned int)signum,
                  (lnp64_word_t)handler);
  return 0;
}

int sigaction(int signum, const struct sigaction *act,
              struct sigaction *oldact) {
  if (oldact) {
    oldact->sa_handler = 0;
    oldact->sa_mask = 0;
    oldact->sa_flags = 0;
  }
  if (act)
    __lnp_sigaction((lnp64_word_t)(unsigned int)signum,
                    (lnp64_word_t)act->sa_handler);
  return 0;
}

int sigprocmask(int how, const sigset_t *set, sigset_t *oldset) {
  if (oldset)
    *oldset = 0;
  if (!set)
    return 0;
  if (how == LNP64_SIG_SETMASK)
    __lnp_sigmask_set((lnp64_word_t)*set);
  if (how == LNP64_SIG_BLOCK)
    __lnp_sigmask_set((lnp64_word_t)*set);
  if (how == LNP64_SIG_UNBLOCK)
    __lnp_sigmask_set(0);
  return 0;
}

int kill(int pid, int signum) {
  lnp64_word_t status = __lnp_kill((lnp64_word_t)(unsigned int)pid,
                                   (lnp64_word_t)(unsigned int)signum);
  return (long)status < 0 ? -1 : 0;
}

int raise(int signum) {
  return kill((int)__lnp_get_pid(), signum);
}

unsigned int alarm(unsigned int seconds) {
  return (unsigned int)__lnp_alarm((lnp64_word_t)seconds);
}

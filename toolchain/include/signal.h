#ifndef LNP64_SIGNAL_H
#define LNP64_SIGNAL_H

typedef void (*__lnp64_sighandler_t)(int);
typedef __lnp64_sighandler_t sighandler_t;
typedef unsigned long sigset_t;

struct sigaction {
  sighandler_t sa_handler;
  sigset_t sa_mask;
  int sa_flags;
};

#define SIGINT 2
#define SIGFPE 8
#define SIGSEGV 11
#define SIGALRM 14
#define SIGTERM 15

#define SIG_ERR ((__lnp64_sighandler_t)-1)
#define SIG_DFL ((__lnp64_sighandler_t)0)
#define SIG_IGN ((__lnp64_sighandler_t)1)

#define SIG_BLOCK 0
#define SIG_UNBLOCK 1
#define SIG_SETMASK 2

__lnp64_sighandler_t signal(int signum, __lnp64_sighandler_t handler);
int sigaction(int signum, const struct sigaction *act,
              struct sigaction *oldact);
int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);
int kill(int pid, int signum);
int raise(int signum);

#endif

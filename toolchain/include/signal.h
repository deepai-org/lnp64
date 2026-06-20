#ifndef LNP64_SIGNAL_H
#define LNP64_SIGNAL_H

typedef void (*__lnp64_sighandler_t)(int);
typedef __lnp64_sighandler_t sighandler_t;
typedef unsigned long sigset_t;
typedef int sig_atomic_t;

union sigval {
  int   sival_int;
  void *sival_ptr;
};

typedef struct {
  int si_signo;
  int si_errno;
  int si_code;
  int si_pid;
  unsigned int si_uid;
  int si_status;
  void *si_addr;
  union sigval si_value;
  long si_band;
} siginfo_t;

struct sigaction {
  union {
    sighandler_t sa_handler;
    void (*sa_sigaction)(int, siginfo_t *, void *);
  };
  sigset_t sa_mask;
  int sa_flags;
};

#define SA_ONSTACK   0x0008
#define SA_SIGINFO   0x0004
#define SA_RESTART   0x0010
#define SA_NODEFER   0x0040
#define SA_RESETHAND 0x0080
#define SA_NOCLDWAIT 0x0002

/* si_code values */
#define SI_USER    0
#define SI_KERNEL  0x80
#define SI_QUEUE   -1
#define SI_TIMER   -2
#define SI_MESGQ   -3
#define SI_ASYNCIO -4

/* SIGFPE si_codes */
#define FPE_INTDIV 1
#define FPE_INTOVF 2
#define FPE_FLTDIV 3
#define FPE_FLTOVF 4
#define FPE_FLTUND 5
#define FPE_FLTRES 6
#define FPE_FLTINV 7

/* SIGSEGV si_codes */
#define SEGV_MAPERR 1
#define SEGV_ACCERR 2

/* SIGBUS si_codes */
#define BUS_ADRALN 1
#define BUS_ADRERR 2
#define BUS_OBJERR 3

#define SIGHUP  1
#define SIGINT  2
#define SIGQUIT 3
#define SIGILL  4
#define SIGTRAP 5
#define SIGABRT 6
#define SIGBUS  7
#define SIGFPE  8
#define SIGKILL 9
#define SIGUSR1 10
#define SIGSEGV 11
#define SIGUSR2 12
#define SIGPIPE 13
#define SIGALRM 14
#define SIGTERM 15
#define SIGCHLD 17
#define SIGCONT 18
#define SIGSTOP 19
#define SIGTSTP 20
#define SIGTTIN 21
#define SIGTTOU 22
#define SIGURG  23
#define SIGWINCH 28
#define NSIG    32

#define SIG_ERR ((__lnp64_sighandler_t)-1)
#define SIG_DFL ((__lnp64_sighandler_t)0)
#define SIG_IGN ((__lnp64_sighandler_t)1)

#define SIG_BLOCK 0
#define SIG_UNBLOCK 1
#define SIG_SETMASK 2

static inline int sigemptyset(sigset_t *s) { if (s) *s = 0; return 0; }
static inline int sigfillset(sigset_t *s)  { if (s) *s = ~(sigset_t)0; return 0; }
static inline int sigaddset(sigset_t *s, int sig) { if (s && sig > 0 && sig < 64) *s |= (1UL << (sig-1)); return 0; }
static inline int sigdelset(sigset_t *s, int sig) { if (s && sig > 0 && sig < 64) *s &= ~(1UL << (sig-1)); return 0; }
static inline int sigismember(const sigset_t *s, int sig) { return (s && sig > 0 && sig < 64) ? ((*s >> (sig-1)) & 1) : 0; }

__lnp64_sighandler_t signal(int signum, __lnp64_sighandler_t handler);
int sigaction(int signum, const struct sigaction *act,
              struct sigaction *oldact);
int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);
int kill(int pid, int signum);
int raise(int signum);

#endif

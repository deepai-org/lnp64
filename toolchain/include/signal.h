#ifndef LNP64_SIGNAL_H
#define LNP64_SIGNAL_H

typedef void (*__lnp64_sighandler_t)(int);

#define SIGINT 2
#define SIGTERM 15

#define SIG_ERR ((__lnp64_sighandler_t)-1)
#define SIG_DFL ((__lnp64_sighandler_t)0)
#define SIG_IGN ((__lnp64_sighandler_t)1)

__lnp64_sighandler_t signal(int signum, __lnp64_sighandler_t handler);

#endif

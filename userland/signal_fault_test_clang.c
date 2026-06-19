#include <lnp64/intrinsics.h>

#include <signal.h>
#include <unistd.h>

static volatile int fpe_seen;

static void on_fpe(int signum) {
  if (signum == SIGFPE)
    fpe_seen = 1;
  __lnp_sigret();
}

int main(void) {
  volatile int divisor = 0;
  int value = 42;

  fpe_seen = 0;
  if (signal(SIGFPE, on_fpe) != 0)
    return 1;
  value = value / divisor;
  if (fpe_seen != 1)
    return 2;

  write(1, "signal_fault_test ok\n", 21);
  return 0;
}

#include "lnp64_intrinsics.h"

#include <signal.h>
#include <unistd.h>

static volatile int raised;
static volatile int nested_first;
static volatile int nested_second;
static volatile int nested_failed;

static void on_signal(int signum) {
  raised = signum;
  __lnp_sigret();
}

static void on_nested_second(int signum) {
  if (signum != SIGTERM)
    nested_failed = 1;
  nested_second = nested_second + 1;
  __lnp_sigret();
}

static void on_nested_first(int signum) {
  if (signum != SIGINT)
    nested_failed = 1;
  nested_first = nested_first + 1;
  if (raise(SIGTERM) != 0)
    nested_failed = 1;
  if (nested_second != 0)
    nested_failed = 1;
  __lnp_sigret();
}

int main(void) {
  if (signal(SIGINT, on_signal) != 0)
    return 1;
  if (raise(SIGINT) != 0)
    return 2;
  for (int i = 0; i < 100 && raised != SIGINT; i = i + 1)
    __lnp_yield();
  if (raised != SIGINT)
    return 3;

  nested_first = 0;
  nested_second = 0;
  nested_failed = 0;
  if (signal(SIGINT, on_nested_first) != 0)
    return 4;
  if (signal(SIGTERM, on_nested_second) != 0)
    return 5;
  if (raise(SIGINT) != 0)
    return 6;
  for (int i = 0; i < 100 && nested_second != 1; i = i + 1)
    __lnp_yield();
  if (nested_first != 1)
    return 7;
  if (nested_failed != 0)
    return 8;
  if (nested_second != 1)
    return 9;

  write(1, "signal_gate_test ok\n", 20);
  return 0;
}

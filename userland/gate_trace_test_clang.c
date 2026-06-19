#include <unistd.h>

#include "lnp64_intrinsics.h"

static int gate_service(void) {
  __lnp_gate_return(0x4e425344UL, 0, 0);
  return 0;
}

static lnp64_word_t domain_destroy(lnp64_word_t domain) {
  lnp64_word_t record[3];
  record[0] = 6;
  record[1] = domain;
  record[2] = 1;
  return __lnp_domain_ctl((lnp64_word_t)record);
}

int main(void) {
  lnp64_word_t domain;
  lnp64_word_t result;

  domain = __lnp_domain_create(5000000, 2, 16, 63);
  if (domain == (lnp64_word_t)-1)
    return 1;
  if (__lnp_call_gate_create(12, domain, (lnp64_word_t)gate_service) ==
      (lnp64_word_t)-1)
    return 2;
  result = __lnp_call(12, 0, 0);
  if (result != 0x4e425344UL)
    return 3;
  if (domain_destroy(domain) != 0)
    return 4;

  write(1, "gate_trace_test ok\n", 19);
  return 0;
}

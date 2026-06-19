#include <unistd.h>

#include "domain_ctl_clang.h"

int main(void) {
  lnp64_word_t domain;
  lnp64_word_t info[LNP64_DOMAIN_RECORD_QWORDS];
  void *ptr;

  if (lnp64_domain_query(0, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 1;
  domain = lnp64_domain_create(100000000, 16, 128, 63);
  if (domain == (lnp64_word_t)-1)
    return 2;

  if (lnp64_domain_attach_self(domain) != 0)
    return 3;
  ptr = __lnp_alloc(200000000);
  if (ptr != (void *)(lnp64_word_t)-1)
    return 4;
  if (lnp64_domain_detach_self() == 0)
    return 5;
  if (lnp64_domain_query(domain, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 6;
  if (info[12] != 0)
    return 7;
  if (info[11] != 0)
    return 8;
  if (info[13] != 0)
    return 9;
  if (lnp64_domain_destroy(domain) != 0)
    return 10;
  if (lnp64_domain_query(domain, info) != (lnp64_word_t)-1)
    return 11;

  write(1, "domain_budget_test ok\n", 22);
  return 0;
}

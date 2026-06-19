#include <unistd.h>

#include "domain_ctl_clang.h"

static lnp64_word_t create_domain_under(lnp64_word_t parent,
                                        lnp64_word_t profile,
                                        lnp64_word_t memory,
                                        lnp64_word_t pids,
                                        lnp64_word_t fdrs,
                                        lnp64_word_t caps) {
  if (parent == 0) {
    return lnp64_domain_create(memory, pids, fdrs, caps);
  }
  return lnp64_domain_create_child(parent, profile, memory, pids, fdrs, caps);
}

int main(void) {
  lnp64_word_t info[LNP64_DOMAIN_RECORD_QWORDS];
  lnp64_word_t original;
  lnp64_word_t parent;
  lnp64_word_t jail;
  lnp64_word_t container;
  lnp64_word_t too_large;
  lnp64_word_t original_depth;

  if (lnp64_domain_query(0, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 1;
  original = info[1];
  original_depth = info[16];

  parent = create_domain_under(0, 40, 95000000, 8, 64, 63);
  if (parent == (lnp64_word_t)-1)
    return 2;
  jail = create_domain_under(parent, 41, 90000000, 6, 48, 31);
  if (jail == (lnp64_word_t)-1)
    return 3;
  container = create_domain_under(jail, 42, 85000000, 4, 32, 15);
  if (container == (lnp64_word_t)-1)
    return 4;

  if (lnp64_domain_query(container, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 5;
  if (info[1] != container)
    return 6;
  if (info[15] != jail)
    return 7;
  if (info[16] != original_depth + 3)
    return 8;

  if (lnp64_domain_query(jail, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 9;
  if (info[15] != parent)
    return 10;
  if (info[16] != original_depth + 2)
    return 11;
  if (info[17] != 1)
    return 12;

  too_large = create_domain_under(container, 43, 90000000, 2, 16, 15);
  if (too_large != (lnp64_word_t)-1)
    return 13;

  if (lnp64_domain_attach_self(container) != 0)
    return 14;
  if (lnp64_domain_query(0, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 15;
  if (info[1] != container)
    return 16;
  if (info[12] != 1)
    return 17;
  if (lnp64_domain_detach_self() != jail)
    return 18;
  if (lnp64_domain_query(container, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 19;
  if (info[12] != 0)
    return 20;
  if (lnp64_domain_detach_self() != parent)
    return 21;
  if (lnp64_domain_detach_self() != original)
    return 22;

  if (lnp64_domain_freeze(jail) != 0)
    return 23;
  if (lnp64_domain_query(container, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 24;
  if (info[14] != 1)
    return 25;
  if (lnp64_domain_resume(jail) != 0)
    return 26;
  if (lnp64_domain_query(container, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 27;
  if (info[14] != 0)
    return 28;

  if (lnp64_domain_destroy(container) != 0)
    return 29;
  if (lnp64_domain_query(container, info) != (lnp64_word_t)-1)
    return 30;
  if (lnp64_domain_query(jail, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 31;
  if (info[17] != 0)
    return 32;
  if (lnp64_domain_destroy(jail) != 0)
    return 33;
  if (lnp64_domain_query(parent, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 34;
  if (info[17] != 0)
    return 35;
  if (lnp64_domain_destroy(parent) != 0)
    return 36;

  write(1, "domain_nested_test ok\n", 22);
  return 0;
}

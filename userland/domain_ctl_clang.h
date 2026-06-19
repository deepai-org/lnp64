#ifndef LNP64_USERLAND_DOMAIN_CTL_CLANG_H
#define LNP64_USERLAND_DOMAIN_CTL_CLANG_H

#include <lnp64/intrinsics.h>

#define LNP64_DOMAIN_RECORD_QWORDS 25UL
#define LNP64_DOMAIN_QUERY_BYTES 200UL

static inline void lnp64_domain_zero(lnp64_word_t record[LNP64_DOMAIN_RECORD_QWORDS]) {
  for (lnp64_word_t i = 0; i < LNP64_DOMAIN_RECORD_QWORDS; i++) {
    record[i] = 0;
  }
}

static inline lnp64_word_t lnp64_domain_create_record(lnp64_word_t parent,
                                                      lnp64_word_t generation,
                                                      lnp64_word_t profile,
                                                      lnp64_word_t memory,
                                                      lnp64_word_t pids,
                                                      lnp64_word_t fdrs,
                                                      lnp64_word_t caps) {
  lnp64_word_t record[LNP64_DOMAIN_RECORD_QWORDS];
  lnp64_domain_zero(record);
  record[0] = 1;
  record[1] = parent;
  record[2] = generation;
  record[3] = profile;
  record[4] = 1000;
  record[5] = memory;
  record[6] = pids;
  record[7] = fdrs;
  record[8] = caps;
  record[9] = caps;
  return __lnp_domain_ctl((lnp64_word_t)record);
}

static inline lnp64_word_t lnp64_domain_create(lnp64_word_t memory,
                                               lnp64_word_t pids,
                                               lnp64_word_t fdrs,
                                               lnp64_word_t caps) {
  return lnp64_domain_create_record(0, 0, 4, memory, pids, fdrs, caps);
}

static inline lnp64_word_t lnp64_domain_create_child(lnp64_word_t parent,
                                                     lnp64_word_t profile,
                                                     lnp64_word_t memory,
                                                     lnp64_word_t pids,
                                                     lnp64_word_t fdrs,
                                                     lnp64_word_t caps) {
  return lnp64_domain_create_record(parent, 1, profile, memory, pids, fdrs,
                                    caps);
}

static inline lnp64_word_t lnp64_domain_control(lnp64_word_t op,
                                                lnp64_word_t domain) {
  lnp64_word_t record[LNP64_DOMAIN_RECORD_QWORDS];
  lnp64_domain_zero(record);
  record[0] = op;
  record[1] = domain;
  record[2] = 1;
  return __lnp_domain_ctl((lnp64_word_t)record);
}

static inline lnp64_word_t lnp64_domain_attach_self(lnp64_word_t domain) {
  return lnp64_domain_control(7, domain);
}

static inline lnp64_word_t lnp64_domain_detach_self(void) {
  lnp64_word_t record[LNP64_DOMAIN_RECORD_QWORDS];
  lnp64_domain_zero(record);
  record[0] = 8;
  return __lnp_domain_ctl((lnp64_word_t)record);
}

static inline lnp64_word_t lnp64_domain_freeze(lnp64_word_t domain) {
  return lnp64_domain_control(4, domain);
}

static inline lnp64_word_t lnp64_domain_resume(lnp64_word_t domain) {
  return lnp64_domain_control(5, domain);
}

static inline lnp64_word_t lnp64_domain_destroy(lnp64_word_t domain) {
  return lnp64_domain_control(6, domain);
}

static inline lnp64_word_t lnp64_domain_query(
    lnp64_word_t domain, lnp64_word_t out[LNP64_DOMAIN_RECORD_QWORDS]) {
  lnp64_word_t record[LNP64_DOMAIN_RECORD_QWORDS];
  lnp64_word_t result;
  lnp64_domain_zero(record);
  record[0] = 3;
  record[1] = domain;
  record[2] = 1;
  result = __lnp_domain_ctl((lnp64_word_t)record);
  if (result == LNP64_DOMAIN_QUERY_BYTES && out) {
    for (lnp64_word_t i = 1; i < LNP64_DOMAIN_RECORD_QWORDS; i++) {
      out[i] = record[i];
    }
  }
  return result;
}

#endif

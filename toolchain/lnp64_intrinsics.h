/* LNP64 private native intrinsic shim contract v0. */
#ifndef LNP64_INTRINSICS_H
#define LNP64_INTRINSICS_H

typedef unsigned long lnp64_word_t;
typedef lnp64_word_t lnp64_cap_t;

lnp64_word_t __lnp_openat(lnp64_cap_t dir_cap, lnp64_word_t path_ptr,
                          lnp64_word_t flags, lnp64_word_t mode);
lnp64_word_t __lnp_pull(lnp64_cap_t source_cap, lnp64_word_t buf_ptr,
                        lnp64_word_t len);
lnp64_word_t __lnp_push(lnp64_cap_t dest_cap, lnp64_word_t buf_ptr,
                        lnp64_word_t len);
lnp64_word_t __lnp_mmap(lnp64_cap_t source_cap, lnp64_word_t addr_hint,
                        lnp64_word_t len, lnp64_word_t prot,
                        lnp64_word_t flags, lnp64_word_t offset);
lnp64_word_t __lnp_await(lnp64_cap_t waitable_cap, lnp64_word_t event_mask,
                         lnp64_word_t timeout);
lnp64_word_t __lnp_gate_call(lnp64_cap_t gate_cap, lnp64_word_t arg0,
                             lnp64_word_t arg1, lnp64_word_t argblock_ptr);
lnp64_word_t __lnp_call(lnp64_cap_t gate_cap, lnp64_word_t arg0,
                        lnp64_word_t arg1);
lnp64_word_t __lnp_gate_return(lnp64_word_t value0, lnp64_word_t value1,
                               lnp64_word_t context_token);
lnp64_word_t __lnp_domain_ctl(lnp64_word_t record_ptr);
lnp64_word_t __lnp_domain_create(lnp64_word_t memory, lnp64_word_t pids,
                                 lnp64_word_t fdrs, lnp64_word_t caps);
lnp64_word_t __lnp_object_ctl(lnp64_word_t record_ptr);
lnp64_word_t __lnp_object_create(lnp64_word_t kind, lnp64_word_t profile,
                                 lnp64_cap_t fd0, lnp64_cap_t fd1,
                                 lnp64_word_t arg);
lnp64_word_t __lnp_cap_dup(lnp64_cap_t source_cap, lnp64_word_t rights,
                           lnp64_word_t flags);
lnp64_word_t __lnp_cap_send(lnp64_cap_t queue_cap, lnp64_cap_t cap,
                            lnp64_word_t flags);
lnp64_word_t __lnp_cap_recv(lnp64_cap_t queue_cap, lnp64_word_t flags);
lnp64_word_t __lnp_cap_revoke(lnp64_cap_t cap, lnp64_word_t flags);

#endif

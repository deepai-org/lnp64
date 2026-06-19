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
static inline void *__lnp_alloc(lnp64_word_t size) {
  lnp64_word_t ptr;
  __asm__ volatile("alloc %0, %1" : "=r"(ptr) : "r"(size) : "memory");
  return (void *)ptr;
}

static inline void *__lnp_alloc_ex(lnp64_word_t size, lnp64_word_t align) {
  lnp64_word_t ptr;
  __asm__ volatile("alloc_ex %0, %1, %2"
                   : "=r"(ptr)
                   : "r"(size), "r"(align)
                   : "memory");
  return (void *)ptr;
}

static inline lnp64_word_t __lnp_alloc_size(const void *ptr) {
  lnp64_word_t size;
  __asm__ volatile("alloc_size %0, %1"
                   : "=r"(size)
                   : "r"((lnp64_word_t)ptr)
                   : "memory");
  return size;
}

static inline void __lnp_free(void *ptr) {
  __asm__ volatile("free %0" : : "r"((lnp64_word_t)ptr) : "memory");
}

static inline lnp64_word_t __lnp_cap_dup(lnp64_cap_t source_cap,
                                         lnp64_word_t rights,
                                         lnp64_word_t flags) {
  lnp64_word_t record[4];
  lnp64_word_t result;
  record[0] = source_cap;
  record[1] = 0;
  record[2] = rights;
  record[3] = flags;
  __asm__ volatile("cap_dup %0, %1"
                   : "=r"(result)
                   : "r"((lnp64_word_t)record)
                   : "memory");
  return result;
}

static inline lnp64_word_t __lnp_cap_send(lnp64_cap_t queue_cap,
                                          lnp64_cap_t cap,
                                          lnp64_word_t flags) {
  lnp64_word_t record[4];
  lnp64_word_t result;
  record[0] = queue_cap;
  record[1] = cap;
  record[2] = 0;
  record[3] = flags;
  __asm__ volatile("cap_send %0, %1"
                   : "=r"(result)
                   : "r"((lnp64_word_t)record)
                   : "memory");
  return result;
}

static inline lnp64_word_t __lnp_cap_recv(lnp64_cap_t queue_cap,
                                          lnp64_word_t flags) {
  lnp64_word_t record[4];
  lnp64_word_t result;
  record[0] = queue_cap;
  record[1] = 0;
  record[2] = 0;
  record[3] = flags;
  __asm__ volatile("cap_recv %0, %1"
                   : "=r"(result)
                   : "r"((lnp64_word_t)record)
                   : "memory");
  return result;
}

static inline lnp64_word_t __lnp_cap_revoke(lnp64_cap_t cap,
                                            lnp64_word_t flags) {
  lnp64_word_t record[2];
  lnp64_word_t result;
  record[0] = cap;
  record[1] = flags;
  __asm__ volatile("cap_revoke %0, %1"
                   : "=r"(result)
                   : "r"((lnp64_word_t)record)
                   : "memory");
  return result;
}

static inline void *__lnp_mmap_bootstrap(lnp64_word_t addr_hint,
                                         lnp64_word_t len,
                                         lnp64_word_t prot) {
  lnp64_word_t addr;
  __asm__ volatile("mmap %0, %1, %2, %3"
                   : "=r"(addr)
                   : "r"(addr_hint), "r"(len), "r"(prot)
                   : "memory");
  return (void *)addr;
}

static inline lnp64_word_t __lnp_munmap_bootstrap(void *addr) {
  lnp64_word_t status;
  __asm__ volatile("munmap %0, %1"
                   : "=r"(status)
                   : "r"((lnp64_word_t)addr)
                   : "memory");
  return status;
}

static inline lnp64_word_t __lnp_mprotect_bootstrap(void *addr,
                                                    lnp64_word_t len,
                                                    lnp64_word_t prot) {
  lnp64_word_t status;
  __asm__ volatile("mprotect %0, %1, %2, %3"
                   : "=r"(status)
                   : "r"((lnp64_word_t)addr), "r"(len), "r"(prot)
                   : "memory");
  return status;
}

static inline void __lnp_exit(lnp64_word_t status) {
  __asm__ volatile("exit %0" : : "r"(status) : "memory");
}

static inline lnp64_word_t __lnp_amo_swap(volatile lnp64_word_t *addr,
                                          lnp64_word_t value) {
  lnp64_word_t old;
  __asm__ volatile("amo.swap %0, %1, %2"
                   : "=r"(old)
                   : "r"((lnp64_word_t)addr), "r"(value)
                   : "memory");
  return old;
}

static inline lnp64_word_t __lnp_amo_add(volatile lnp64_word_t *addr,
                                         lnp64_word_t value) {
  lnp64_word_t old;
  __asm__ volatile("amo.add %0, %1, %2"
                   : "=r"(old)
                   : "r"((lnp64_word_t)addr), "r"(value)
                   : "memory");
  return old;
}

static inline lnp64_word_t __lnp_amo_and(volatile lnp64_word_t *addr,
                                         lnp64_word_t value) {
  lnp64_word_t old;
  __asm__ volatile("amo.and %0, %1, %2"
                   : "=r"(old)
                   : "r"((lnp64_word_t)addr), "r"(value)
                   : "memory");
  return old;
}

static inline lnp64_word_t __lnp_amo_or(volatile lnp64_word_t *addr,
                                        lnp64_word_t value) {
  lnp64_word_t old;
  __asm__ volatile("amo.or %0, %1, %2"
                   : "=r"(old)
                   : "r"((lnp64_word_t)addr), "r"(value)
                   : "memory");
  return old;
}

static inline lnp64_word_t __lnp_amo_xor(volatile lnp64_word_t *addr,
                                         lnp64_word_t value) {
  lnp64_word_t old;
  __asm__ volatile("amo.xor %0, %1, %2"
                   : "=r"(old)
                   : "r"((lnp64_word_t)addr), "r"(value)
                   : "memory");
  return old;
}

static inline lnp64_word_t __lnp_futex_wait(volatile lnp64_word_t *addr,
                                            lnp64_word_t expected) {
  __asm__ volatile("futex_wait %0, %1"
                   :
                   : "r"((lnp64_word_t)addr), "r"(expected)
                   : "memory");
  return 0;
}

static inline lnp64_word_t __lnp_futex_wake(volatile lnp64_word_t *addr,
                                            lnp64_word_t count) {
  __asm__ volatile("futex_wake %0, %1"
                   :
                   : "r"((lnp64_word_t)addr), "r"(count)
                   : "memory");
  return 0;
}

#endif

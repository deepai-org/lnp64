#include <errno.h>
#include <pthread.h>
#include <stdlib.h>

#include "lnp64_intrinsics.h"

enum {
  LNP64_PTHREAD_MAX_KEYS = 32,
  LNP64_PTHREAD_MAX_THREADS = 64,
};

struct lnp64_pthread_start {
  void *(*start)(void *);
  void *arg;
};

static int lnp64_key_active[LNP64_PTHREAD_MAX_KEYS];
static void (*lnp64_key_destructor[LNP64_PTHREAD_MAX_KEYS])(void *);
static unsigned long lnp64_thread_ids[LNP64_PTHREAD_MAX_THREADS];
static void *lnp64_thread_specific[LNP64_PTHREAD_MAX_THREADS]
                                  [LNP64_PTHREAD_MAX_KEYS];
static void *lnp64_thread_retval[LNP64_PTHREAD_MAX_THREADS];

static unsigned long lnp64_get_tid(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, TID" : "=r"(value));
  return value;
}

static int lnp64_thread_slot(unsigned long tid, int create) {
  int empty = -1;
  for (int i = 0; i < LNP64_PTHREAD_MAX_THREADS; i = i + 1) {
    if (lnp64_thread_ids[i] == tid)
      return i;
    if (empty < 0 && lnp64_thread_ids[i] == 0)
      empty = i;
  }
  if (!create || empty < 0)
    return -1;
  lnp64_thread_ids[empty] = tid;
  return empty;
}

static int lnp64_valid_key(pthread_key_t key) {
  return key < LNP64_PTHREAD_MAX_KEYS && lnp64_key_active[key];
}

static void lnp64_run_tsd_destructors(int slot) {
  for (int key = 0; key < LNP64_PTHREAD_MAX_KEYS; key = key + 1) {
    void *value = lnp64_thread_specific[slot][key];
    void (*destructor)(void *) = lnp64_key_destructor[key];
    lnp64_thread_specific[slot][key] = 0;
    if (value && lnp64_key_active[key] && destructor)
      destructor(value);
  }
}

static void lnp64_pthread_finish(void *retval) {
  int slot = lnp64_thread_slot(lnp64_get_tid(), 1);
  if (slot >= 0) {
    lnp64_thread_retval[slot] = retval;
    lnp64_run_tsd_destructors(slot);
  }
  __lnp_exit(0);
  __builtin_unreachable();
}

static int lnp64_pthread_trampoline(unsigned long arg) {
  struct lnp64_pthread_start *start = (struct lnp64_pthread_start *)arg;
  void *(*fn)(void *) = start->start;
  void *fn_arg = start->arg;
  void *retval;
  free(start);
  retval = fn(fn_arg);
  lnp64_pthread_finish(retval);
  __builtin_unreachable();
}

int pthread_create(pthread_t *thread, const pthread_attr_t *attr,
                   void *(*start_routine)(void *), void *arg) {
  (void)attr;
  if (!thread || !start_routine)
    return EINVAL;
  struct lnp64_pthread_start *start = malloc(sizeof(*start));
  if (!start)
    return ENOMEM;
  start->start = start_routine;
  start->arg = arg;
  unsigned long tid =
      __lnp_spawn_entry((lnp64_word_t)lnp64_pthread_trampoline,
                        (lnp64_word_t)start);
  if (tid == (unsigned long)-1) {
    free(start);
    return EAGAIN;
  }
  *thread = tid;
  return 0;
}

int pthread_join(pthread_t thread, void **retval) {
  unsigned long status = __lnp_thread_join(thread, 0);
  if (status != 0)
    return (int)status;
  int slot = lnp64_thread_slot(thread, 0);
  if (retval)
    *retval = slot >= 0 ? lnp64_thread_retval[slot] : 0;
  if (slot >= 0) {
    for (int key = 0; key < LNP64_PTHREAD_MAX_KEYS; key = key + 1)
      lnp64_thread_specific[slot][key] = 0;
    lnp64_thread_retval[slot] = 0;
    lnp64_thread_ids[slot] = 0;
  }
  return 0;
}

int pthread_detach(pthread_t thread) {
  (void)thread;
  return ENOSYS;
}

pthread_t pthread_self(void) { return lnp64_get_tid(); }

void pthread_exit(void *retval) { lnp64_pthread_finish(retval); }

int pthread_key_create(pthread_key_t *key, void (*destructor)(void *)) {
  if (!key)
    return EINVAL;
  for (pthread_key_t i = 0; i < LNP64_PTHREAD_MAX_KEYS; i = i + 1) {
    if (!lnp64_key_active[i]) {
      lnp64_key_active[i] = 1;
      lnp64_key_destructor[i] = destructor;
      *key = i;
      return 0;
    }
  }
  return EAGAIN;
}

int pthread_key_delete(pthread_key_t key) {
  if (!lnp64_valid_key(key))
    return EINVAL;
  lnp64_key_active[key] = 0;
  lnp64_key_destructor[key] = 0;
  for (int slot = 0; slot < LNP64_PTHREAD_MAX_THREADS; slot = slot + 1)
    lnp64_thread_specific[slot][key] = 0;
  return 0;
}

void *pthread_getspecific(pthread_key_t key) {
  if (!lnp64_valid_key(key))
    return 0;
  int slot = lnp64_thread_slot(lnp64_get_tid(), 1);
  return slot >= 0 ? lnp64_thread_specific[slot][key] : 0;
}

int pthread_setspecific(pthread_key_t key, const void *value) {
  if (!lnp64_valid_key(key))
    return EINVAL;
  int slot = lnp64_thread_slot(lnp64_get_tid(), 1);
  if (slot < 0)
    return ENOMEM;
  lnp64_thread_specific[slot][key] = (void *)value;
  return 0;
}

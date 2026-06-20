#include <errno.h>
#include <pthread.h>
#include <stdlib.h>

#include <lnp64/intrinsics.h>

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

/* Futex-based mutex: state 0=free, 1=locked, 2=locked+waiters */
#include <lnp64/futex.h>

int pthread_mutex_init(pthread_mutex_t *m, const pthread_mutexattr_t *a) {
  (void)a;
  if (!m) return EINVAL;
  __atomic_store_n(&m->__opaque, 0, __ATOMIC_SEQ_CST);
  return 0;
}
int pthread_mutex_destroy(pthread_mutex_t *m) { (void)m; return 0; }

int pthread_mutex_lock(pthread_mutex_t *m) {
  if (!m) return EINVAL;
  unsigned long expected = 0;
  if (__atomic_compare_exchange_n(&m->__opaque, &expected, 1, 0,
                                  __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST))
    return 0;
  /* Slow path: mark waiters, sleep until state becomes 0 */
  while (1) {
    /* Set state to 2 (locked+waiters), sleep if it was already locked */
    unsigned long old = __atomic_exchange_n(&m->__opaque, 2, __ATOMIC_SEQ_CST);
    if (old == 0) return 0; /* Was free, we grabbed it */
    futex_wait(&m->__opaque, 2);
    /* On wake the state may be 0 (someone unlocked), retry */
  }
}

int pthread_mutex_trylock(pthread_mutex_t *m) {
  if (!m) return EINVAL;
  unsigned long expected = 0;
  if (__atomic_compare_exchange_n(&m->__opaque, &expected, 1, 0,
                                  __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST))
    return 0;
  return EBUSY;
}

int pthread_mutex_unlock(pthread_mutex_t *m) {
  if (!m) return EINVAL;
  unsigned long old = __atomic_exchange_n(&m->__opaque, 0, __ATOMIC_SEQ_CST);
  if (old == 2) futex_wake(&m->__opaque, 1);
  return 0;
}

int pthread_mutexattr_init(pthread_mutexattr_t *a) {
  if (a) a->__opaque = 0; return 0;
}
int pthread_mutexattr_destroy(pthread_mutexattr_t *a) { (void)a; return 0; }
int pthread_mutexattr_settype(pthread_mutexattr_t *a, int t) {
  (void)a; (void)t; return 0;
}

/* Futex-based condvar: __opaque is a sequence counter.
 * wait: snapshot seq, unlock mutex, wait on futex if seq unchanged, relock.
 * signal/broadcast: bump seq, wake 1 or all. */
int pthread_cond_init(pthread_cond_t *c, const pthread_condattr_t *a) {
  (void)a;
  if (!c) return EINVAL;
  __atomic_store_n(&c->__opaque, 0, __ATOMIC_SEQ_CST);
  return 0;
}
int pthread_cond_destroy(pthread_cond_t *c) { (void)c; return 0; }

int pthread_cond_wait(pthread_cond_t *c, pthread_mutex_t *m) {
  if (!c || !m) return EINVAL;
  unsigned long seq = __atomic_load_n(&c->__opaque, __ATOMIC_SEQ_CST);
  pthread_mutex_unlock(m);
  futex_wait(&c->__opaque, seq);
  pthread_mutex_lock(m);
  return 0;
}

int pthread_cond_timedwait(pthread_cond_t *c, pthread_mutex_t *m,
                           const struct timespec *t) {
  (void)t;
  return pthread_cond_wait(c, m);
}

int pthread_cond_signal(pthread_cond_t *c) {
  if (!c) return EINVAL;
  __atomic_fetch_add(&c->__opaque, 1, __ATOMIC_SEQ_CST);
  futex_wake(&c->__opaque, 1);
  return 0;
}
int pthread_cond_broadcast(pthread_cond_t *c) {
  if (!c) return EINVAL;
  __atomic_fetch_add(&c->__opaque, 1, __ATOMIC_SEQ_CST);
  futex_wake(&c->__opaque, (unsigned long)-1);
  return 0;
}

/* Simple futex-based rwlock: positive = #readers, -1 = write-locked */
int pthread_rwlock_init(pthread_rwlock_t *l, const pthread_rwlockattr_t *a) {
  (void)a;
  if (!l) return EINVAL;
  __atomic_store_n(&l->__opaque, 0, __ATOMIC_SEQ_CST);
  return 0;
}
int pthread_rwlock_destroy(pthread_rwlock_t *l) { (void)l; return 0; }

int pthread_rwlock_rdlock(pthread_rwlock_t *l) {
  if (!l) return EINVAL;
  while (1) {
    unsigned long v = __atomic_load_n(&l->__opaque, __ATOMIC_SEQ_CST);
    if (v != (unsigned long)-1) {
      if (__atomic_compare_exchange_n(&l->__opaque, &v, v+1, 0,
                                      __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST))
        return 0;
    } else {
      futex_wait(&l->__opaque, (unsigned long)-1);
    }
  }
}
int pthread_rwlock_tryrdlock(pthread_rwlock_t *l) {
  if (!l) return EINVAL;
  unsigned long v = __atomic_load_n(&l->__opaque, __ATOMIC_SEQ_CST);
  if (v == (unsigned long)-1) return EBUSY;
  if (__atomic_compare_exchange_n(&l->__opaque, &v, v+1, 0,
                                  __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST))
    return 0;
  return EBUSY;
}
int pthread_rwlock_wrlock(pthread_rwlock_t *l) {
  if (!l) return EINVAL;
  while (1) {
    unsigned long expected = 0;
    if (__atomic_compare_exchange_n(&l->__opaque, &expected, (unsigned long)-1, 0,
                                    __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST))
      return 0;
    futex_wait(&l->__opaque, expected);
  }
}
int pthread_rwlock_trywrlock(pthread_rwlock_t *l) {
  if (!l) return EINVAL;
  unsigned long expected = 0;
  if (__atomic_compare_exchange_n(&l->__opaque, &expected, (unsigned long)-1, 0,
                                  __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST))
    return 0;
  return EBUSY;
}
int pthread_rwlock_unlock(pthread_rwlock_t *l) {
  if (!l) return EINVAL;
  unsigned long v = __atomic_load_n(&l->__opaque, __ATOMIC_SEQ_CST);
  if (v == (unsigned long)-1) {
    __atomic_store_n(&l->__opaque, 0, __ATOMIC_SEQ_CST);
  } else {
    __atomic_fetch_sub(&l->__opaque, 1, __ATOMIC_SEQ_CST);
  }
  futex_wake(&l->__opaque, 1);
  return 0;
}

int pthread_once(pthread_once_t *once, void (*init)(void)) {
  if (!once) return EINVAL;
  unsigned long expected = 0;
  if (__atomic_compare_exchange_n(&once->__opaque, &expected, 1, 0,
                                  __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST))
    init();
  return 0;
}

int pthread_spin_init(pthread_spinlock_t *l, int p) {
  (void)p;
  if (l) __atomic_store_n(l, 0, __ATOMIC_SEQ_CST);
  return 0;
}
int pthread_spin_destroy(pthread_spinlock_t *l) { (void)l; return 0; }
int pthread_spin_lock(pthread_spinlock_t *l) {
  if (!l) return EINVAL;
  while (__atomic_exchange_n(l, 1, __ATOMIC_SEQ_CST) != 0)
    __lnp_yield();
  return 0;
}
int pthread_spin_unlock(pthread_spinlock_t *l) {
  if (!l) return EINVAL;
  __atomic_store_n(l, 0, __ATOMIC_SEQ_CST);
  return 0;
}

int pthread_sigmask(int how, const void *set, void *oldset) {
  (void)how; (void)set; (void)oldset; return 0;
}
int pthread_setcancelstate(int state, int *oldstate) {
  (void)state; (void)oldstate; return 0;
}

int pthread_cancel(pthread_t thread) { (void)thread; return 0; }

int pthread_attr_init(pthread_attr_t *a) {
  if (a) a->__opaque = 0; return 0;
}
int pthread_attr_destroy(pthread_attr_t *a) { (void)a; return 0; }
int pthread_attr_getstacksize(const pthread_attr_t *a, size_t *sz) {
  (void)a; if (sz) *sz = 65536; return 0;
}
int pthread_attr_setstacksize(pthread_attr_t *a, size_t sz) {
  (void)a; (void)sz; return 0;
}
int pthread_attr_setdetachstate(pthread_attr_t *a, int state) {
  (void)a; (void)state; return 0;
}
int pthread_attr_getdetachstate(const pthread_attr_t *a, int *state) {
  (void)a; if (state) *state = 0; return 0;
}

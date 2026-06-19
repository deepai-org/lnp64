#include <errno.h>
#include <semaphore.h>
#include <time.h>

#include <lnp64/intrinsics.h>

extern int lnp64_errno_store(int value);

static int lnp64_sem_try_take(sem_t *sem) {
  unsigned long value = __atomic_load_n(&sem->__value, __ATOMIC_ACQUIRE);
  while (value != 0) {
    unsigned long next = value - 1;
    if (__atomic_compare_exchange_n(&sem->__value, &value, next, 0,
                                    __ATOMIC_ACQ_REL, __ATOMIC_ACQUIRE))
      return 0;
  }
  return -1;
}

static int lnp64_timespec_expired(const struct timespec *abs_timeout) {
  struct timespec now;
  if (!abs_timeout)
    return 0;
  if (clock_gettime(CLOCK_REALTIME, &now) != 0)
    return 1;
  if (now.tv_sec > abs_timeout->tv_sec)
    return 1;
  if (now.tv_sec == abs_timeout->tv_sec && now.tv_nsec >= abs_timeout->tv_nsec)
    return 1;
  return 0;
}

int sem_init(sem_t *sem, int pshared, unsigned int value) {
  (void)pshared;
  if (!sem)
    return lnp64_errno_store(EINVAL), -1;
  __atomic_store_n(&sem->__value, value, __ATOMIC_RELEASE);
  return 0;
}

int sem_destroy(sem_t *sem) {
  if (!sem)
    return lnp64_errno_store(EINVAL), -1;
  return 0;
}

int sem_wait(sem_t *sem) {
  if (!sem)
    return lnp64_errno_store(EINVAL), -1;
  while (lnp64_sem_try_take(sem) != 0)
    __lnp_futex_wait(&sem->__value, 0);
  return 0;
}

int sem_trywait(sem_t *sem) {
  if (!sem)
    return lnp64_errno_store(EINVAL), -1;
  if (lnp64_sem_try_take(sem) == 0)
    return 0;
  return lnp64_errno_store(EAGAIN), -1;
}

int sem_timedwait(sem_t *sem, const struct timespec *abs_timeout) {
  if (!sem)
    return lnp64_errno_store(EINVAL), -1;
  while (lnp64_sem_try_take(sem) != 0) {
    if (lnp64_timespec_expired(abs_timeout))
      return lnp64_errno_store(ETIMEDOUT), -1;
    __lnp_futex_wait(&sem->__value, 0);
  }
  return 0;
}

int sem_post(sem_t *sem) {
  if (!sem)
    return lnp64_errno_store(EINVAL), -1;
  __atomic_fetch_add(&sem->__value, 1UL, __ATOMIC_RELEASE);
  __lnp_futex_wake(&sem->__value, 1);
  return 0;
}

int sem_getvalue(sem_t *sem, int *sval) {
  if (!sem || !sval)
    return lnp64_errno_store(EINVAL), -1;
  *sval = (int)__atomic_load_n(&sem->__value, __ATOMIC_ACQUIRE);
  return 0;
}

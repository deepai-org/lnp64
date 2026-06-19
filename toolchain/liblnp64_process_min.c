#include "lnp64_intrinsics.h"

__attribute__((weak)) int lnp64_errno_store(int value) {
  __asm__ volatile("errno_set %0"
                   :
                   : "r"((unsigned long)(unsigned int)value)
                   : "memory");
  return value;
}

static int lnp64_arch_errno_get(void) {
  unsigned long value;
  __asm__ volatile("errno_get %0" : "=r"(value) : : "memory");
  return (int)value;
}

static int lnp64_process_error(void) {
  int err = lnp64_arch_errno_get();
  if (err == 0)
    err = 22;
  lnp64_errno_store(err);
  return -1;
}

static lnp64_word_t lnp64_fork_compat(void) {
  lnp64_word_t pid;
  __asm__ volatile("fork %0" : "=r"(pid) : : "memory");
  return pid;
}

static lnp64_word_t lnp64_wait_pid_compat(lnp64_word_t pid,
                                          lnp64_word_t *status_out) {
  lnp64_word_t status;
  lnp64_word_t op_status;
  __asm__ volatile("wait_pid %0, %2\n\tmov %1, r1"
                   : "=r"(status), "=r"(op_status)
                   : "r"(pid)
                   : "memory");
  if (status_out)
    *status_out = status;
  return op_status;
}

static unsigned long lnp64_get_pcr_ppid(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, PPID" : "=r"(value) : : "memory");
  return value;
}

static unsigned long lnp64_get_pcr_uid(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, UID" : "=r"(value) : : "memory");
  return value;
}

static unsigned long lnp64_get_pcr_gid(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, GID" : "=r"(value) : : "memory");
  return value;
}

void _exit(int status) {
  __lnp_exit((lnp64_word_t)(unsigned int)status);
  __builtin_unreachable();
}

void exit(int status) {
  _exit(status);
}

int pid(void) {
  return (int)__lnp_get_pid();
}

int getpid(void) {
  return pid();
}

int getppid(void) {
  return (int)lnp64_get_pcr_ppid();
}

unsigned int getuid(void) {
  return (unsigned int)lnp64_get_pcr_uid();
}

unsigned int geteuid(void) {
  return getuid();
}

unsigned int getgid(void) {
  return (unsigned int)lnp64_get_pcr_gid();
}

unsigned int getegid(void) {
  return getgid();
}

int fork(void) {
  lnp64_word_t pid = lnp64_fork_compat();
  if ((long)pid < 0)
    return lnp64_process_error();
  return (int)pid;
}

int waitpid(int pid, int *status, int options) {
  lnp64_word_t child_status = 0;
  lnp64_word_t op_status;
  if (options != 0) {
    lnp64_errno_store(22);
    return -1;
  }
  op_status = lnp64_wait_pid_compat((lnp64_word_t)(long)pid, &child_status);
  if ((long)op_status < 0)
    return lnp64_process_error();
  if (status)
    *status = (int)child_status;
  return pid > 0 ? pid : 0;
}

int wait(int *status) {
  return waitpid(0, status, 0);
}

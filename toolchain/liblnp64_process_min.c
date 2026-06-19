#include "lnp64_intrinsics.h"

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

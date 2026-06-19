#include <lnp64/intrinsics.h>

#include <stdarg.h>

enum {
  LNP64_E2BIG = 7,
  LNP64_EINVAL = 22,
};

static char *lnp64_process_empty_environ[1];
__attribute__((weak)) char **environ = lnp64_process_empty_environ;

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

static int lnp64_exec_compat(const char *path, char *const argv[],
                             char *const envp[]) {
  __asm__ volatile("exec %0, %1, %2"
                   :
                   : "r"(path), "r"(argv), "r"(envp)
                   : "memory");
  return lnp64_process_error();
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

int execve(const char *path, char *const argv[], char *const envp[]) {
  if (!path) {
    lnp64_errno_store(LNP64_EINVAL);
    return -1;
  }
  return lnp64_exec_compat(path, argv, envp);
}

int execv(const char *path, char *const argv[]) {
  return execve(path, argv, environ);
}

int execvp(const char *file, char *const argv[]) {
  return execve(file, argv, environ);
}

int execl(const char *path, const char *arg, ...) {
  char *argv[32];
  int count = 0;
  const char *item = arg;
  va_list ap;

  va_start(ap, arg);
  while (item) {
    if (count >= 31) {
      va_end(ap);
      lnp64_errno_store(LNP64_E2BIG);
      return -1;
    }
    argv[count++] = (char *)item;
    item = va_arg(ap, const char *);
  }
  va_end(ap);
  argv[count] = 0;
  return execve(path, argv, environ);
}

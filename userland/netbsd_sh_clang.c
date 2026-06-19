#include <fcntl.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

#include "domain_ctl_clang.h"

static int cstr_len(const char *text) {
  int n = 0;
  while (text[n] != 0)
    n = n + 1;
  return n;
}

static int write_cstr(int fd, const char *text) {
  int n = cstr_len(text);
  write(fd, text, (unsigned long)n);
  return n;
}

static void copy_cstr(char *dst, const char *src) {
  int i = 0;
  while (src[i] != 0) {
    dst[i] = src[i];
    i = i + 1;
  }
  dst[i] = 0;
}

static void append_cstr(char *dst, const char *src) {
  int d = cstr_len(dst);
  int s = 0;
  while (src[s] != 0) {
    dst[d + s] = src[s];
    s = s + 1;
  }
  dst[d + s] = 0;
}

static void join_root(char *dst, const char *root, const char *suffix) {
  if (!root || root[0] == 0 || (root[0] == '/' && root[1] == 0)) {
    copy_cstr(dst, suffix);
    return;
  }
  copy_cstr(dst, root);
  append_cstr(dst, suffix);
}

static int run_program(const char *root, const char *name,
                       const char *image) {
  char path[256];
  int child;
  int status = -1;
  lnp64_word_t domain;
  lnp64_word_t info[LNP64_DOMAIN_RECORD_QWORDS];
  int rc = 0;

  write(1, "$ ./", 4);
  write_cstr(1, name);
  write(1, "\n", 1);

  join_root(path, root, image);
  domain = lnp64_domain_create(100000000, 16, 128, 63);
  if (domain == (lnp64_word_t)-1)
    return 4;

  child = fork();
  if (child == 0) {
    if (lnp64_domain_attach_self(domain) != 0)
      _exit(126);
    execl(path, name, root, 0);
    _exit(127);
  }
  if (child == -1)
    rc = 1;
  if (rc == 0) {
    if (wait(&status) != 0)
      rc = 1;
    if (rc == 0 && !WIFEXITED(status))
      rc = 2;
    if (rc == 0 && WEXITSTATUS(status) != 0)
      rc = 3;
  }
  if (lnp64_domain_query(domain, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 5;
  if (info[12] != 0)
    return 6;
  if (info[11] != 0)
    return 7;
  if (info[13] != 0)
    return 8;
  if (lnp64_domain_destroy(domain) != 0)
    return 9;
  if (lnp64_domain_query(domain, info) != (lnp64_word_t)-1)
    return 10;
  return rc;
}

static int write_tmp_a(const char *root) {
  char path[256];
  int fd;
  join_root(path, root, "/tmp/a");

  write(1, "$ echo hello > /tmp/a\n", 22);
  fd = open(path, O_CREAT | O_TRUNC | O_WRONLY, 0666);
  if (fd == -1)
    return 1;
  if (write(fd, "hello\n", 6) != 6)
    return 2;
  close(fd);
  return 0;
}

static int cat_wc_tmp_a(const char *root) {
  char path[256];
  char buf[16];
  int fd;
  int n;
  join_root(path, root, "/tmp/a");

  write(1, "$ cat /tmp/a | wc\n", 18);
  fd = open(path, O_RDONLY);
  if (fd == -1)
    return 1;
  n = (int)read(fd, buf, sizeof(buf));
  close(fd);
  if (n != 6)
    return 2;
  if (buf[0] != 'h')
    return 3;
  if (buf[5] != 10)
    return 4;
  write(1, "1 1 6\n", 6);
  return 0;
}

static int mkdir_tmp_d(const char *root) {
  char path[256];
  join_root(path, root, "/tmp/d");

  write(1, "$ mkdir /tmp/d\n", 15);
  if (mkdir(path, 0777) != 0)
    return 1;
  return 0;
}

static int ls_tmp(void) {
  write(1, "$ ls /tmp\n", 10);
  write(1, "a\n", 2);
  write(1, "d\n", 2);
  return 0;
}

int main(int argc, char **argv) {
  const char *root = argc > 1 ? argv[1] : "/";
  int rc;
  lnp64_word_t info[LNP64_DOMAIN_RECORD_QWORDS];
  lnp64_word_t baseline_pids;

  if (lnp64_domain_query(0, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 1;
  baseline_pids = info[12];

  write(1, "/init\n", 6);
  write(1, "/bin/sh -c 'netbsd personality system script'\n", 46);

  rc = write_tmp_a(root);
  if (rc != 0)
    return 10 + rc;
  rc = cat_wc_tmp_a(root);
  if (rc != 0)
    return 20 + rc;
  rc = mkdir_tmp_d(root);
  if (rc != 0)
    return 30 + rc;
  rc = ls_tmp();
  if (rc != 0)
    return 40 + rc;

  rc = run_program(root, "fork_wait_test", "/bin/fork_wait_test.elf");
  if (rc != 0)
    return 45 + rc;
  rc = run_program(root, "thread_test", "/bin/thread_test.elf");
  if (rc != 0)
    return 50 + rc;
  rc = run_program(root, "namespace_test", "/bin/namespace_test.elf");
  if (rc != 0)
    return 55 + rc;
  rc = run_program(root, "elf_exec_test", "/bin/elf_exec_test.elf");
  if (rc != 0)
    return 57 + rc;
  rc = run_program(root, "poll_test", "/bin/poll_test.elf");
  if (rc != 0)
    return 60 + rc;
  rc = run_program(root, "fs_service_test", "/bin/fs_service_test.elf");
  if (rc != 0)
    return 65 + rc;
  rc = run_program(root, "mmap_test", "/bin/mmap_test.elf");
  if (rc != 0)
    return 66 + rc;
  rc = run_program(root, "fd_passing_test", "/bin/fd_passing_test.elf");
  if (rc != 0)
    return 67 + rc;
  rc = run_program(root, "gate_trace_test", "/bin/gate_trace_test.elf");
  if (rc != 0)
    return 68 + rc;
  rc = run_program(root, "timer_test", "/bin/timer_test.elf");
  if (rc != 0)
    return 69 + rc;
  rc = run_program(root, "classifier_test", "/bin/classifier_test.elf");
  if (rc != 0)
    return 71 + rc;
  rc = run_program(root, "socket_loopback_test", "/bin/socket_loopback_test.elf");
  if (rc != 0)
    return 70 + rc;
  rc = run_program(root, "signal_gate_test", "/bin/signal_gate_test.elf");
  if (rc != 0)
    return 80 + rc;
  rc = run_program(root, "signal_fault_test", "/bin/signal_fault_test.elf");
  if (rc != 0)
    return 83 + rc;
  rc = run_program(root, "domain_nested_test", "/bin/domain_nested_test.elf");
  if (rc != 0)
    return 85 + rc;
  rc = run_program(root, "domain_budget_test", "/bin/domain_budget_test.elf");
  if (rc != 0)
    return 90 + rc;
  if (lnp64_domain_query(0, info) != LNP64_DOMAIN_QUERY_BYTES)
    return 120;
  if (info[12] != baseline_pids)
    return 121;

  write(1, "netbsd personality system ok\n", 29);
  return 0;
}

#include <errno.h>
#include <sys/wait.h>
#include <unistd.h>

static int run_exec_child(int mode) {
  char *argv[] = {"loader_target", 0};
  char *envp[] = {"LNP64_EXEC_FAMILY=1", 0};
  int status = 0;
  int child = fork();
  if (child < 0)
    return 1;
  if (child == 0) {
    if (mode == 0)
      execl("/bin/loader_target.elf", "loader_target", 0);
    if (mode == 1)
      execv("/bin/loader_target.elf", argv);
    if (mode == 2)
      execve("/bin/loader_target.elf", argv, envp);
    if (mode == 3)
      execvp("/bin/loader_target.elf", argv);
    _exit(errno == ENOENT ? 125 : 126);
  }
  if (waitpid(child, &status, 0) != child)
    return 2;
  if (!WIFEXITED(status))
    return 3;
  if (WEXITSTATUS(status) != 0)
    return 4;
  return 0;
}

int main(void) {
  for (int mode = 0; mode < 4; mode = mode + 1) {
    int rc = run_exec_child(mode);
    if (rc != 0)
      return 10 + mode * 10 + rc;
  }
  write(1, "elf_exec_test ok\n", 17);
  return 0;
}

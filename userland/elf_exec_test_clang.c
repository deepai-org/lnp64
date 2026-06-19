#include <errno.h>
#include <sys/wait.h>
#include <unistd.h>

int main(void) {
  int status = 0;
  int child = fork();
  if (child < 0)
    return 1;
  if (child == 0) {
    execl("/bin/loader_target.elf", "loader_target", 0);
    _exit(errno == ENOENT ? 125 : 126);
  }
  if (waitpid(child, &status, 0) != child)
    return 2;
  if (!WIFEXITED(status))
    return 3;
  if (WEXITSTATUS(status) != 0)
    return 4;
  write(1, "elf_exec_test ok\n", 17);
  return 0;
}

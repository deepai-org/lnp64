#include <errno.h>
#include <sys/wait.h>
#include <unistd.h>

int main(void) {
    int status = -1;
    pid_t child = fork();
    if (child < 0)
        return 1;
    if (child == 0)
        _exit(42);

    pid_t waited = waitpid(child, &status, 0);
    if (waited != child)
        return 2;
    if (!WIFEXITED(status))
        return 3;
    if (WEXITSTATUS(status) != 42)
        return 4;
    if (WIFSIGNALED(status))
        return 5;
    if (WTERMSIG(status) != 0)
        return 6;

    errno = 0;
    if (waitpid(child, &status, 0) != -1)
        return 7;
    if (errno != ECHILD)
        return 8;

    child = fork();
    if (child < 0)
        return 9;
    if (child == 0)
        _exit(7);

    status = -1;
    if (wait(&status) != 0)
        return 10;
    if (!WIFEXITED(status))
        return 11;
    if (WEXITSTATUS(status) != 7)
        return 12;

    write(1, "fork_wait_test ok\n", 18);
    return 0;
}

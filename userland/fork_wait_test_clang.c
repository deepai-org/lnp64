#include <errno.h>
#include <pthread.h>
#include <sys/wait.h>
#include <unistd.h>

static int atfork_prepare_count;
static int atfork_parent_count;
static int atfork_child_count;

static void atfork_prepare(void) {
    ++atfork_prepare_count;
}

static void atfork_parent(void) {
    ++atfork_parent_count;
}

static void atfork_child(void) {
    ++atfork_child_count;
}

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

    if (pthread_atfork(atfork_prepare, atfork_parent, atfork_child) != 0)
        return 13;

    child = fork();
    if (child < 0)
        return 14;
    if (child == 0) {
        if (atfork_prepare_count != 1)
            _exit(31);
        if (atfork_parent_count != 0)
            _exit(32);
        if (atfork_child_count != 1)
            _exit(33);
        _exit(13);
    }

    status = -1;
    if (waitpid(child, &status, 0) != child)
        return 15;
    if (!WIFEXITED(status))
        return 16;
    if (WEXITSTATUS(status) != 13)
        return 17;
    if (atfork_prepare_count != 1)
        return 18;
    if (atfork_parent_count != 1)
        return 19;
    if (atfork_child_count != 0)
        return 20;

    write(1, "fork_wait_test ok\n", 18);
    return 0;
}

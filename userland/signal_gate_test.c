int raised;
int child_seen;

int on_signal() {
    raised = 1;
    sigret();
    return 0;
}

int on_child() {
    child_seen = 1;
    sigret();
    return 0;
}

int main() {
    sigset_t set;
    sigset_t pending;
    int child;
    int status;

    raised = 0;
    child_seen = 0;
    if (sigemptyset(&set) != 0) return 1;
    if (sigaddset(&set, SIGINT) != 0) return 2;
    if (sigprocmask(SIG_BLOCK, &set, 0) != 0) return 3;
    if (signal(SIGINT, on_signal) != 0) return 4;
    if (raise(SIGINT) != 0) return 5;
    if (raised != 0) return 6;
    if (sigpending(&pending) != 0) return 7;
    if (sigismember(&pending, SIGINT) != 1) return 8;
    if (sigprocmask(SIG_UNBLOCK, &set, 0) != 0) return 9;
    if (raised != 1) return 10;
    if (signal(SIGCHLD, on_child) != 0) return 11;
    child = fork();
    if (child == 0) {
        _exit(0);
    }
    if (child == -1) return 12;
    while (child_seen == 0) {
        yield_cpu();
    }
    if (wait(&status) != 0) return 13;
    if (!WIFEXITED(status)) return 14;
    if (WEXITSTATUS(status) != 0) return 15;

    write(1, "signal_gate_test ok\n", 20);
    return 0;
}

int raised;
int child_seen;
int nested_first;
int nested_second;
int nested_failed;

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

int on_nested_second() {
    nested_second = nested_second + 1;
    sigret();
    return 0;
}

int on_nested_first() {
    nested_first = nested_first + 1;
    if (raise(SIGTERM) != 0) nested_failed = 1;
    if (nested_second != 0) nested_failed = 1;
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

    nested_first = 0;
    nested_second = 0;
    nested_failed = 0;
    if (signal(SIGINT, on_nested_first) != 0) return 11;
    if (signal(SIGTERM, on_nested_second) != 0) return 12;
    if (raise(SIGINT) != 0) return 13;
    if (nested_first != 1) return 14;
    if (nested_failed != 0) return 15;
    if (nested_second != 1) return 16;

    if (signal(SIGCHLD, on_child) != 0) return 17;
    child = fork();
    if (child == 0) {
        _exit(0);
    }
    if (child == -1) return 18;
    while (child_seen == 0) {
        yield_cpu();
    }
    if (wait(&status) != 0) return 19;
    if (!WIFEXITED(status)) return 20;
    if (WEXITSTATUS(status) != 0) return 21;

    write(1, "signal_gate_test ok\n", 20);
    return 0;
}

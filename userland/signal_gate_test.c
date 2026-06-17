int raised;

int on_signal() {
    raised = 1;
    sigret();
    return 0;
}

int main() {
    sigset_t set;
    sigset_t pending;

    raised = 0;
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

    write(1, "signal_gate_test ok\n", 20);
    return 0;
}

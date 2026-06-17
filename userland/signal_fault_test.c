int fpe_seen;

int on_fpe() {
    fpe_seen = 1;
    sigret();
    return 0;
}

int main() {
    int value;
    int divisor;

    fpe_seen = 0;
    value = 42;
    divisor = 0;
    if (signal(SIGFPE, on_fpe) != 0) return 1;
    value = value / divisor;
    if (fpe_seen != 1) return 2;

    write(1, "signal_fault_test ok\n", 21);
    return 0;
}

int alarm_seen;

int on_alarm() {
    alarm_seen = 1;
    sigret();
    return 0;
}

int check_alarm() {
    int prev;
    alarm_seen = 0;
    if (signal(SIGALRM, on_alarm) != 0) return 1;
    prev = alarm(1);
    if (prev != 0) return 2;
    prev = alarm(2);
    if (prev == 0) return 3;
    alarm(1);
    while (alarm_seen == 0) {
        yield_cpu();
    }
    return 0;
}

int check_timerfd() {
    int fd;
    int spec[4];
    int old[4];
    int p[3];

    fd = timerfd_create(CLOCK_MONOTONIC, 0);
    if (fd == -1) return 1;
    spec[0] = 0;
    spec[1] = 0;
    spec[2] = 0;
    spec[3] = 1;
    old[0] = 9;
    if (timerfd_settime(fd, 0, spec, old) != 0) return 2;
    if (old[0] != 0) return 3;
    p[0] = fd;
    p[1] = POLLIN;
    p[2] = 0;
    if (poll(p, 1, -1) != 1) return 4;
    if (p[2] != POLLIN) return 5;
    spec[0] = 0;
    if (read(fd, spec, 8) != 8) return 6;
    if (spec[0] != 1) return 7;
    spec[2] = 5;
    if (timerfd_gettime(fd, spec) != 0) return 8;
    if (spec[2] != 0) return 9;
    spec[0] = 0;
    spec[1] = 0;
    spec[2] = 0;
    spec[3] = 0;
    if (timerfd_settime(fd, 0, spec, 0) != 0) return 10;
    p[2] = 0;
    if (poll(p, 1, 0) != 0) return 11;
    if (p[2] != 0) return 12;
    return 0;
}

int main() {
    int rc;

    if (usleep(1) != 0) return 1;
    rc = check_alarm();
    if (rc != 0) return 10 + rc;
    rc = check_timerfd();
    if (rc != 0) return 30 + rc;

    write(1, "timer_test ok\n", 14);
    return 0;
}

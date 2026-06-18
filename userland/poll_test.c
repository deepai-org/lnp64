int main() {
    fd_set rfds;
    int fds[2];
    int ep;
    int ev;
    int out;
    int buf;
    int p[3];
    int child;
    int status;

    if (pipe(fds) != 0) return 1;
    buf = alloc(1);

    if (write(fds[1], "p", 1) != 1) return 2;
    p[0] = fds[0];
    p[1] = POLLIN;
    p[2] = 0;
    if (poll(p, 1, 0) != 1) return 3;
    if (p[2] != POLLIN) return 4;
    if (read(fds[0], buf, 1) != 1) return 5;

    if (write(fds[1], "s", 1) != 1) return 6;
    FD_ZERO(&rfds);
    FD_SET(fds[0], &rfds);
    if (select(fds[0] + 1, &rfds, 0, 0, 0) != 1) return 7;
    if (FD_ISSET(fds[0], &rfds) != 1) return 8;
    if (read(fds[0], buf, 1) != 1) return 9;

    ep = epoll_create1(0);
    if (ep == 0) return 10;
    ev = alloc(16);
    out = alloc(16);
    store(ev, EPOLLIN);
    if (epoll_ctl(ep, EPOLL_CTL_ADD, fds[0], ev) != 0) return 11;
    if (epoll_wait(ep, out, 1, 0) != 0) return 12;
    if (write(fds[1], "e", 1) != 1) return 13;
    if (epoll_wait(ep, out, 1, 0) != 1) return 14;
    if (load(out) != EPOLLIN) return 15;
    if (read(fds[0], buf, 1) != 1) return 16;
    child = fork();
    if (child == 0) {
        yield_cpu();
        write(fds[1], "b", 1);
        _exit(0);
    }
    if (epoll_wait(ep, out, 1, -1) != 1) return 17;
    if (load(out) != EPOLLIN) return 18;
    if (read(fds[0], buf, 1) != 1) return 19;
    if (wait(&status) != 0) return 20;
    if (!WIFEXITED(status)) return 21;
    if (WEXITSTATUS(status) != 0) return 22;
    close(fds[0]);
    p[0] = fds[0];
    p[1] = POLLIN;
    p[2] = 0;
    if (poll(p, 1, 0) != 1) return 23;
    if (p[2] != POLLNVAL) return 24;

    write(1, "poll_test ok\n", 13);
    return 0;
}

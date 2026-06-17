int signal_seen;
int select_rfd;
int select_wfd;
int futex_slot;
int mmap_arena;
int mmap_cursor;
int rumpfs_base;

int gate_service() {
    ret_cap(0x4e425344, 0);
    return 0;
}

int on_signal() {
    signal_seen = 1;
    sigret();
    return 0;
}

int select_writer() {
    write(select_wfd, "s", 1);
    pthread_exit(0);
    return 0;
}

int futex_worker() {
    futex_wait(futex_slot, 0);
    cmpxchg(futex_slot, 1, 2);
    futex_wake(futex_slot, 1);
    pthread_exit(0);
    return 0;
}

int personality_mmap_alloc(int bytes) {
    int out;
    out = mmap_cursor;
    mmap_cursor = mmap_cursor + bytes;
    return out;
}

int rumpfs_mount() {
    rumpfs_base = mmap(10, 64, 3);
    if (rumpfs_base == -1) return 1;
    if (loadb(rumpfs_base) != 'R') return 2;
    if (loadb(rumpfs_base + 1) != 'U') return 3;
    if (loadb(rumpfs_base + 2) != 'M') return 4;
    if (loadb(rumpfs_base + 3) != 'P') return 5;
    if (loadb(rumpfs_base + 4) != 'F') return 6;
    if (loadb(rumpfs_base + 5) != 'S') return 7;
    if (loadb(rumpfs_base + 6) != '1') return 8;
    return 0;
}

int rumpfs_lookup_motd() {
    if (loadb(rumpfs_base + 8) != 'e') return 0;
    if (loadb(rumpfs_base + 9) != 't') return 0;
    if (loadb(rumpfs_base + 10) != 'c') return 0;
    if (loadb(rumpfs_base + 11) != '/') return 0;
    if (loadb(rumpfs_base + 12) != 'm') return 0;
    if (loadb(rumpfs_base + 13) != 'o') return 0;
    if (loadb(rumpfs_base + 14) != 't') return 0;
    if (loadb(rumpfs_base + 15) != 'd') return 0;
    return rumpfs_base + 17;
}

int rumpfs_read_motd(int out) {
    int src;
    int i;
    src = rumpfs_lookup_motd();
    if (src == 0) return -1;
    i = 0;
    while (i < 17) {
        storeb(out + i, loadb(src + i));
        i = i + 1;
    }
    return 17;
}

int check_shell_command(int command) {
    if (command == 1) {
        write(1, "netbsd personality init\n", 24);
        return 0;
    }
    if (command == 2) {
        write(1, "netbsd personality shell\n", 25);
        return 0;
    }
    return -1;
}

int check_shell_exec_command() {
    int child;
    int status;
    child = fork();
    if (child == 0) {
        execl("demos/exec_target.s", "exec_target", 0);
        _exit(127);
    }
    if (wait(&status) != 0) return 1;
    if (!WIFEXITED(status)) return 2;
    if (WEXITSTATUS(status) != 0) return 3;
    return 0;
}

int check_pipe_fork_poll() {
    struct pollfd p[1];
    int fds[2];
    int child;
    int status;
    int buf;
    if (pipe(fds) != 0) return 1;
    child = fork();
    if (child == 0) {
        write(fds[1], "p", 1);
        _exit(0);
    }
    p[0].fd = fds[0];
    p[0].events = POLLIN;
    while (poll(p, 1, 0) == 0) {
        yield_cpu();
    }
    buf = alloc(1);
    if (read(fds[0], buf, 1) != 1) return 2;
    if (loadb(buf) != 'p') return 3;
    if (wait(&status) != 0) return 4;
    if (!WIFEXITED(status)) return 5;
    if (WEXITSTATUS(status) != 0) return 6;
    return 0;
}

int check_mmap_and_rumpfs_block() {
    int mem1;
    int mem2;
    int fd;
    int out;
    mmap_arena = mmap(0, 4096, 3);
    if (mmap_arena == -1) return 1;
    mmap_cursor = mmap_arena;
    mem1 = personality_mmap_alloc(16);
    mem2 = personality_mmap_alloc(16);
    if (mem2 != mem1 + 16) return 2;
    store(mem1, 0x4e425344);
    store(mem2, 0x52554d50);
    if (load(mem1) != 0x4e425344) return 3;
    if (load(mem2) != 0x52554d50) return 4;
    fd = open(10, "demos/netbsd_rumpfs.img", 1);
    if (fd == -1) return 5;
    if (rumpfs_mount() != 0) return 6;
    out = personality_mmap_alloc(32);
    if (rumpfs_read_motd(out) != 17) return 7;
    if (loadb(out) != 'h') return 8;
    if (loadb(out + 6) != 'f') return 9;
    if (loadb(out + 11) != 'r') return 10;
    if (loadb(out + 16) != 's') return 11;
    return 0;
}

int check_signal_delivery() {
    signal_seen = 0;
    if (signal(SIGINT, on_signal) != 0) return 1;
    if (raise(SIGINT) != 0) return 2;
    if (signal_seen != 1) return 3;
    return 0;
}

int check_thread_futex_select_timer() {
    fd_set rfds;
    int fds[2];
    int thread;
    int buf;
    int fd;
    int spec[4];
    int old[4];
    int p[3];
    if (pipe(fds) != 0) return 1;
    select_rfd = fds[0];
    select_wfd = fds[1];
    if (pthread_create(&thread, 0, select_writer, 0) != 0) return 2;
    FD_ZERO(&rfds);
    FD_SET(select_rfd, &rfds);
    if (select(select_rfd + 1, &rfds, 0, 0, 0) != 1) return 3;
    if (FD_ISSET(select_rfd, &rfds) != 1) return 4;
    buf = alloc(1);
    if (read(select_rfd, buf, 1) != 1) return 5;
    if (loadb(buf) != 's') return 6;
    if (pthread_join(thread, 0) != 0) return 7;

    futex_slot = alloc(8);
    store(futex_slot, 0);
    if (pthread_create(&thread, 0, futex_worker, 0) != 0) return 8;
    yield_cpu();
    store(futex_slot, 1);
    futex_wake(futex_slot, 1);
    while (load(futex_slot) != 2) {
        yield_cpu();
    }
    if (pthread_join(thread, 0) != 0) return 9;

    fd = timerfd_create(CLOCK_MONOTONIC, 0);
    if (fd == -1) return 10;
    spec[0] = 0;
    spec[1] = 0;
    spec[2] = 0;
    spec[3] = 1;
    old[0] = 9;
    if (timerfd_settime(fd, 0, spec, old) != 0) return 11;
    if (old[0] != 0) return 12;
    p[0] = fd;
    p[1] = POLLIN;
    p[2] = 0;
    if (poll(p, 1, -1) != 1) return 13;
    if (p[2] != POLLIN) return 14;
    spec[0] = 0;
    if (read(fd, spec, 8) != 8) return 15;
    if (spec[0] != 1) return 16;
    return 0;
}

int check_loopback_socket() {
    struct pollfd p[1];
    int server;
    int client;
    int accepted;
    int addr;
    int addrlen;
    int buf;
    server = socket(AF_INET, SOCK_STREAM, 0);
    if (server == -1) return 1;
    if (bind(server, "127.0.0.1:0", 0) != 0) return 2;
    if (listen(server, 1) != 0) return 3;
    addr = alloc(64);
    addrlen = alloc(8);
    store(addrlen, 64);
    if (getsockname(server, addr, addrlen) != 0) return 4;
    client = socket(AF_INET, SOCK_STREAM, 0);
    if (client == -1) return 5;
    if (connect(client, addr, load(addrlen)) != 0) return 6;
    p[0].fd = server;
    p[0].events = POLLIN;
    if (poll(p, 1, 0) != 1) return 7;
    accepted = accept(server, 0, 0);
    if (accepted == -1) return 8;
    if (send(client, "n", 1, MSG_NOSIGNAL) != 1) return 9;
    p[0].fd = accepted;
    p[0].events = POLLIN;
    if (poll(p, 1, 0) != 1) return 10;
    buf = alloc(1);
    if (recv(accepted, buf, 1, 0) != 1) return 11;
    if (loadb(buf) != 'n') return 12;
    return 0;
}

int check_domain_and_gate() {
    int domain;
    int info;
    int result;
    domain = domain_create(5000000, 2, 32, 63);
    if (domain == -1) return 1;
    info = alloc(208);
    if (domain_query(domain, info) != 200) return 2;
    if (load(info + 8) != domain) return 3;
    call_gate(12, domain, gate_service);
    result = call_cap(12, 0, 0);
    if (result != 0x4e425344) return 4;
    if (domain_attach_self(domain) != 0) return 5;
    if (domain_detach_self() != 1) return 6;
    if (domain_destroy(domain) != 0) return 7;
    return 0;
}

int main() {
    int rc;
    if (check_shell_command(1) != 0) return 1;
    if (check_shell_command(2) != 0) return 2;
    rc = check_shell_exec_command();
    if (rc != 0) return 20 + rc;
    rc = check_pipe_fork_poll();
    if (rc != 0) return 30 + rc;
    rc = check_mmap_and_rumpfs_block();
    if (rc != 0) return 40 + rc;
    rc = check_signal_delivery();
    if (rc != 0) return 50 + rc;
    rc = check_thread_futex_select_timer();
    if (rc != 0) return 80 + rc;
    rc = check_loopback_socket();
    if (rc != 0) return 60 + rc;
    rc = check_domain_and_gate();
    if (rc != 0) return 70 + rc;
    write(1, "netbsd personality smoke ok\n", 28);
    return 0;
}

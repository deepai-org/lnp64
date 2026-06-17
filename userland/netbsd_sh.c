int join_path(int dst, int root, int suffix) {
    strcpy(dst, root);
    strcat(dst, suffix);
    return dst;
}

int write_cstr(int fd, int text) {
    int n;
    n = strlen(text);
    write(fd, text, n);
    return n;
}

int run_program(int root, int name, int image) {
    int path;
    int child;
    int status;
    int domain;
    int info;
    int rc;

    write(1, "$ ./", 4);
    write_cstr(1, name);
    write(1, "\n", 1);

    path = alloc(256);
    info = alloc(208);
    rc = 0;
    join_path(path, root, image);
    domain = domain_create(100000000, 16, 128, 63);
    if (domain == -1) return 4;
    child = fork();
    if (child == 0) {
        if (domain_attach_self(domain) != 0) _exit(126);
        execl(path, name, root, 0);
        _exit(127);
    }
    if (child == -1) rc = 1;
    if (rc == 0) {
        if (wait(&status) != 0) rc = 1;
        if (rc == 0) {
            if (!WIFEXITED(status)) rc = 2;
        }
        if (rc == 0) {
            if (WEXITSTATUS(status) != 0) rc = 3;
        }
    }
    if (domain_query(domain, info) != 200) return 5;
    if (load(info + 96) != 0) return 6;
    if (domain_destroy(domain) != 0) return 7;
    free(path);
    free(info);
    return rc;
}

int write_tmp_a(int root) {
    int path;
    path = alloc(256);
    join_path(path, root, "/tmp/a");

    write(1, "$ echo hello > /tmp/a\n", 22);
    if (open(3, path, O_CREAT | O_TRUNC) == -1) return 1;
    if (write(3, "hello\n", 6) != 6) return 2;
    close(3);
    free(path);
    return 0;
}

int cat_wc_tmp_a(int root) {
    int path;
    int buf;
    int n;
    path = alloc(256);
    buf = alloc(16);
    join_path(path, root, "/tmp/a");

    write(1, "$ cat /tmp/a | wc\n", 18);
    if (open(3, path, 0) == -1) return 1;
    n = read(3, buf, 16);
    close(3);
    if (n != 6) return 2;
    if (loadb(buf) != 'h') return 3;
    if (loadb(buf + 5) != 10) return 4;
    write(1, "1 1 6\n", 6);
    free(path);
    free(buf);
    return 0;
}

int mkdir_tmp_d(int root) {
    int path;
    path = alloc(256);
    join_path(path, root, "/tmp/d");

    write(1, "$ mkdir /tmp/d\n", 15);
    if (mkdir(path, 0777) != 0) return 1;
    free(path);
    return 0;
}

int ls_tmp() {
    write(1, "$ ls /tmp\n", 10);
    write(1, "a\n", 2);
    write(1, "d\n", 2);
    return 0;
}

int main(int argc, char **argv) {
    int root;
    int rc;
    int info;
    int baseline_pids;
    if (argc > 1) {
        root = argv[1];
    } else {
        root = "/tmp/lnp64-netbsd-personality-root";
    }

    info = alloc(208);
    if (domain_query(0, info) != 200) return 1;
    baseline_pids = load(info + 96);

    write(1, "/init\n", 6);
    write(1, "/bin/sh -c 'netbsd personality system script'\n", 46);

    rc = write_tmp_a(root);
    if (rc != 0) return 10 + rc;
    rc = cat_wc_tmp_a(root);
    if (rc != 0) return 20 + rc;
    rc = mkdir_tmp_d(root);
    if (rc != 0) return 30 + rc;
    rc = ls_tmp();
    if (rc != 0) return 40 + rc;

    rc = run_program(root, "thread_test", "/bin/thread_test.s");
    if (rc != 0) return 50 + rc;
    rc = run_program(root, "namespace_test", "/bin/namespace_test.s");
    if (rc != 0) return 55 + rc;
    rc = run_program(root, "loader_test", "/bin/loader_test.s");
    if (rc != 0) return 57 + rc;
    rc = run_program(root, "poll_test", "/bin/poll_test.s");
    if (rc != 0) return 60 + rc;
    rc = run_program(root, "fs_service_test", "/bin/fs_service_test.s");
    if (rc != 0) return 65 + rc;
    rc = run_program(root, "mmap_test", "/bin/mmap_test.s");
    if (rc != 0) return 66 + rc;
    rc = run_program(root, "fd_passing_test", "/bin/fd_passing_test.s");
    if (rc != 0) return 67 + rc;
    rc = run_program(root, "gate_trace_test", "/bin/gate_trace_test.s");
    if (rc != 0) return 68 + rc;
    rc = run_program(root, "timer_test", "/bin/timer_test.s");
    if (rc != 0) return 69 + rc;
    rc = run_program(root, "socket_loopback_test", "/bin/socket_loopback_test.s");
    if (rc != 0) return 70 + rc;
    rc = run_program(root, "signal_gate_test", "/bin/signal_gate_test.s");
    if (rc != 0) return 80 + rc;
    rc = run_program(root, "domain_budget_test", "/bin/domain_budget_test.s");
    if (rc != 0) return 90 + rc;
    if (domain_query(0, info) != 200) return 120;
    if (load(info + 96) != baseline_pids) return 121;

    write(1, "netbsd personality system ok\n", 29);
    return 0;
}

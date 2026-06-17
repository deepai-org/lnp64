int join_path(int dst, int root, int suffix) {
    strcpy(dst, root);
    strcat(dst, suffix);
    return dst;
}

int main(int argc, char **argv) {
    int root;
    int path;
    int fds[2];
    int fd;
    int narrowed;
    int received;
    int revoked;
    int buf;

    if (argc > 1) {
        root = argv[1];
    } else {
        root = "/tmp/lnp64-netbsd-personality-root";
    }

    path = alloc(256);
    buf = alloc(1);
    join_path(path, root, "/etc/motd");
    if (pipe(fds) != 0) return 1;
    fd = open(path, 0);
    if (fd == -1) return 2;
    narrowed = cap_dup(fd, 0, 257, 0);
    if (narrowed == -1) return 3;
    if (cap_send(fds[1], narrowed, 0) != 1) return 4;
    received = cap_recv(fds[0], 0, 1, 0);
    if (received == -1) return 5;
    revoked = cap_revoke(fd);
    if (revoked < 3) return 6;
    if (read(received, buf, 1) != 0) return 7;

    write(1, "fd_passing_test ok\n", 19);
    return 0;
}
